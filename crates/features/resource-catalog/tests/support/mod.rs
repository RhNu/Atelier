use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_resource_catalog::{
    BlobId, BlobWriteIntent, BuildVariantRequest, BuiltResourceVariant, ReleaseOutcome,
    ResourceBlobStore, ResourceCatalogError, ResourceCatalogRepository, ResourceCatalogTransaction,
    ResourceCleanupCandidate, ResourceId, ResourceLink, ResourceMetadata, ResourceOwner,
    ResourceRecord, ResourceRef, ResourceResult, ResourceState, ResourceVariant,
    ResourceVariantBuilder, StagedBlob, StagedBlobToken, VariantId,
};

#[derive(Clone, Default)]
pub struct FakeRepository {
    state: Arc<Mutex<FakeRepositoryState>>,
    failures: HashSet<FakeFailure>,
    fail_commit_after: Option<usize>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum FakeFailure {
    Begin,
    Commit,
    MarkReady,
    InsertVariant,
}

#[derive(Default)]
struct FakeRepositoryState {
    records: HashMap<ResourceId, ResourceRecord>,
    links: HashSet<ResourceLink>,
    variants: HashMap<VariantId, ResourceVariant>,
    orphan_blobs: HashSet<BlobId>,
    commit_count: usize,
}

impl FakeRepository {
    pub fn failing_begin() -> Self {
        Self::with_failure(FakeFailure::Begin)
    }

    pub fn failing_commit() -> Self {
        Self::with_failure(FakeFailure::Commit)
    }

    pub fn failing_commit_after_first_commit() -> Self {
        Self {
            fail_commit_after: Some(1),
            ..Self::default()
        }
    }

    pub fn failing_mark_ready() -> Self {
        Self::with_failure(FakeFailure::MarkReady)
    }

    pub fn failing_variant_insert() -> Self {
        Self::with_failure(FakeFailure::InsertVariant)
    }

    pub fn records(&self) -> HashMap<ResourceId, ResourceRecord> {
        self.state.lock().unwrap().records.clone()
    }

    fn with_failure(failure: FakeFailure) -> Self {
        Self {
            failures: HashSet::from([failure]),
            ..Self::default()
        }
    }
}

#[async_trait]
impl ResourceCatalogRepository for FakeRepository {
    async fn begin_transaction(&self) -> ResourceResult<Box<dyn ResourceCatalogTransaction>> {
        if self.failures.contains(&FakeFailure::Begin) {
            return Err(ResourceCatalogError::repository("begin failed"));
        }
        Ok(Box::new(FakeTransaction {
            state: self.state.clone(),
            failures: self.failures.clone(),
            fail_commit_after: self.fail_commit_after,
            pending_records: HashMap::new(),
            links_to_attach: HashSet::new(),
            links_to_detach: HashSet::new(),
            variants: HashMap::new(),
            ready_records: HashSet::new(),
            delete_pending: HashSet::new(),
            orphan_markers_to_clear: HashSet::new(),
        }))
    }

    async fn get_ready_record(&self, id: &ResourceId) -> ResourceResult<Option<ResourceRecord>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .records
            .get(id)
            .filter(|record| record.state == ResourceState::Ready)
            .cloned())
    }

    async fn list_ready_refs_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceRef>> {
        Ok(self
            .list_ready_links_by_owner(owner)
            .await?
            .into_iter()
            .map(|link| ResourceRef::base(link.resource_id))
            .collect())
    }

    async fn list_ready_links_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceLink>> {
        let mut links = {
            let state = self.state.lock().unwrap();
            state
                .links
                .iter()
                .filter(|link| &link.owner == owner)
                .filter(|link| {
                    state
                        .records
                        .get(&link.resource_id)
                        .is_some_and(|record| record.state == ResourceState::Ready)
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        links.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));
        Ok(links)
    }

    async fn get_variant(&self, id: &VariantId) -> ResourceResult<Option<ResourceVariant>> {
        Ok(self.state.lock().unwrap().variants.get(id).cloned())
    }

    async fn list_delete_pending_resources(&self) -> ResourceResult<Vec<ResourceCleanupCandidate>> {
        let mut candidates = {
            let state = self.state.lock().unwrap();
            state
                .records
                .values()
                .filter(|record| record.state == ResourceState::DeletePending)
                .map(|record| ResourceCleanupCandidate {
                    record: record.clone(),
                    variants: state
                        .variants
                        .values()
                        .filter(|variant| variant.resource_id == record.id)
                        .cloned()
                        .collect(),
                })
                .collect::<Vec<_>>()
        };
        candidates.sort_by(|left, right| left.record.id.cmp(&right.record.id));
        Ok(candidates)
    }

    async fn blob_is_referenced_outside_resource(
        &self,
        resource_id: &ResourceId,
        blob_id: &BlobId,
    ) -> ResourceResult<bool> {
        let state = self.state.lock().unwrap();
        Ok(state
            .records
            .values()
            .any(|record| &record.id != resource_id && &record.blob_id == blob_id)
            || state
                .variants
                .values()
                .any(|variant| &variant.resource_id != resource_id && &variant.blob_id == blob_id))
    }

    async fn delete_resource_record_if_unowned(&self, id: &ResourceId) -> ResourceResult<bool> {
        let mut state = self.state.lock().unwrap();
        if state.links.iter().any(|link| &link.resource_id == id) {
            return Ok(false);
        }
        state.records.remove(id);
        state.links.retain(|link| &link.resource_id != id);
        state
            .variants
            .retain(|_, variant| &variant.resource_id != id);
        drop(state);
        Ok(true)
    }

    async fn blob_is_referenced(&self, blob_id: &BlobId) -> ResourceResult<bool> {
        let state = self.state.lock().unwrap();
        Ok(state
            .records
            .values()
            .any(|record| &record.blob_id == blob_id)
            || state
                .variants
                .values()
                .any(|variant| &variant.blob_id == blob_id))
    }

    async fn scan_orphan_blobs(&self) -> ResourceResult<Vec<BlobId>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .orphan_blobs
            .iter()
            .cloned()
            .collect())
    }

    async fn record_orphan_blob(&self, blob_id: &BlobId) -> ResourceResult<()> {
        self.state
            .lock()
            .unwrap()
            .orphan_blobs
            .insert(blob_id.clone());
        Ok(())
    }
}

struct FakeTransaction {
    state: Arc<Mutex<FakeRepositoryState>>,
    failures: HashSet<FakeFailure>,
    fail_commit_after: Option<usize>,
    pending_records: HashMap<ResourceId, ResourceRecord>,
    links_to_attach: HashSet<ResourceLink>,
    links_to_detach: HashSet<ResourceLink>,
    variants: HashMap<VariantId, ResourceVariant>,
    ready_records: HashSet<ResourceId>,
    delete_pending: HashSet<ResourceId>,
    orphan_markers_to_clear: HashSet<BlobId>,
}

#[async_trait]
impl ResourceCatalogTransaction for FakeTransaction {
    async fn insert_pending_record(&mut self, record: ResourceRecord) -> ResourceResult<()> {
        self.pending_records.insert(record.id.clone(), record);
        Ok(())
    }

    async fn attach_owner(&mut self, link: ResourceLink) -> ResourceResult<()> {
        self.links_to_attach.insert(link);
        Ok(())
    }

    async fn detach_owner(&mut self, link: &ResourceLink) -> ResourceResult<()> {
        self.links_to_detach.insert(link.clone());
        Ok(())
    }

    async fn count_owner_links(&self, id: &ResourceId) -> ResourceResult<usize> {
        let persisted = {
            let state = self.state.lock().unwrap();
            state
                .links
                .iter()
                .filter(|link| &link.resource_id == id && !self.links_to_detach.contains(*link))
                .count()
        };
        let attached = self
            .links_to_attach
            .iter()
            .filter(|link| &link.resource_id == id)
            .count();
        Ok(persisted + attached)
    }

    async fn mark_ready(&mut self, id: &ResourceId) -> ResourceResult<()> {
        if self.failures.contains(&FakeFailure::MarkReady) {
            return Err(ResourceCatalogError::repository("mark ready failed"));
        }
        self.ready_records.insert(id.clone());
        Ok(())
    }

    async fn mark_delete_pending(&mut self, id: &ResourceId) -> ResourceResult<()> {
        self.delete_pending.insert(id.clone());
        Ok(())
    }

    async fn insert_variant(&mut self, variant: ResourceVariant) -> ResourceResult<()> {
        if self.failures.contains(&FakeFailure::InsertVariant) {
            return Err(ResourceCatalogError::repository("insert variant failed"));
        }
        self.variants.insert(variant.id.clone(), variant);
        Ok(())
    }

    async fn clear_orphan_blob_marker(&mut self, blob_id: &BlobId) -> ResourceResult<()> {
        self.orphan_markers_to_clear.insert(blob_id.clone());
        Ok(())
    }

    async fn commit(self: Box<Self>) -> ResourceResult<()> {
        if self.should_fail_commit() {
            return Err(ResourceCatalogError::repository("commit failed"));
        }

        let mut state = self.state.lock().unwrap();
        state.commit_count += 1;
        for mut record in self.pending_records.into_values() {
            if self.ready_records.contains(&record.id) {
                record.state = ResourceState::Ready;
            }
            state.records.insert(record.id.clone(), record);
        }
        for id in self.delete_pending {
            if let Some(record) = state.records.get_mut(&id) {
                record.state = ResourceState::DeletePending;
            }
        }
        for link in self.links_to_detach {
            state.links.remove(&link);
        }
        state.links.extend(self.links_to_attach);
        state.variants.extend(self.variants);
        for blob_id in self.orphan_markers_to_clear {
            state.orphan_blobs.remove(&blob_id);
        }
        drop(state);
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> ResourceResult<()> {
        Ok(())
    }
}

impl FakeTransaction {
    fn should_fail_commit(&self) -> bool {
        let persists_new_blob_reference =
            !self.pending_records.is_empty() || !self.variants.is_empty();
        if self.failures.contains(&FakeFailure::Commit) && persists_new_blob_reference {
            return true;
        }
        persists_new_blob_reference
            && self
                .fail_commit_after
                .is_some_and(|count| self.state.lock().unwrap().commit_count >= count)
    }
}

#[derive(Clone, Default)]
pub struct FakeBlobStore {
    state: Arc<Mutex<FakeBlobState>>,
    fail_finalize: bool,
}

#[derive(Default)]
struct FakeBlobState {
    next: u32,
    operations: Vec<String>,
    staged: HashSet<StagedBlobToken>,
    finalized: HashSet<BlobId>,
}

impl FakeBlobStore {
    pub fn failing_finalize() -> Self {
        Self {
            state: Arc::default(),
            fail_finalize: true,
        }
    }

    pub fn operations(&self) -> Vec<String> {
        self.state.lock().unwrap().operations.clone()
    }

    pub fn finalized_blobs(&self) -> Vec<BlobId> {
        let mut blobs = self
            .state
            .lock()
            .unwrap()
            .finalized
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        blobs.sort();
        blobs
    }
}

#[async_trait]
impl ResourceBlobStore for FakeBlobStore {
    async fn stage_blob(&self, intent: BlobWriteIntent) -> ResourceResult<StagedBlob> {
        let BlobWriteIntent::Bytes(bytes) = intent;
        let (token, blob_id) = {
            let mut state = self.state.lock().unwrap();
            state.next += 1;
            state.operations.push(format!("stage:{}", bytes.len()));
            let token = StagedBlobToken::new(format!("staged-{}", state.next));
            let blob_id = BlobId::new(format!("blob-{}", state.next));
            state.staged.insert(token.clone());
            drop(state);
            (token, blob_id)
        };
        Ok(StagedBlob {
            token,
            blob_id,
            metadata: ResourceMetadata {
                byte_size: Some(bytes.len() as u64),
                ..ResourceMetadata::default()
            },
        })
    }

    async fn finalize_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        let mut state = self.state.lock().unwrap();
        state
            .operations
            .push(format!("finalize:{}", staged.as_str()));
        if self.fail_finalize {
            drop(state);
            return Err(ResourceCatalogError::blob_store("finalize failed"));
        }
        state.staged.remove(staged);
        let suffix = staged.as_str().trim_start_matches("staged-");
        state
            .finalized
            .insert(BlobId::new(format!("blob-{suffix}")));
        drop(state);
        Ok(())
    }

    async fn abort_staged_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        let mut state = self.state.lock().unwrap();
        state.operations.push(format!("abort:{}", staged.as_str()));
        state.staged.remove(staged);
        drop(state);
        Ok(())
    }

    async fn delete_blob(&self, blob_id: &BlobId) -> ResourceResult<()> {
        self.state.lock().unwrap().finalized.remove(blob_id);
        Ok(())
    }

    async fn blob_exists(&self, blob_id: &BlobId) -> ResourceResult<bool> {
        Ok(self.state.lock().unwrap().finalized.contains(blob_id))
    }
}

#[derive(Clone, Default)]
pub struct FakeVariantBuilder {
    requests: Arc<Mutex<Vec<BuildVariantRequest>>>,
}

impl FakeVariantBuilder {
    pub fn requests(&self) -> Vec<BuildVariantRequest> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl ResourceVariantBuilder for FakeVariantBuilder {
    async fn build_variant(
        &self,
        request: BuildVariantRequest,
    ) -> ResourceResult<BuiltResourceVariant> {
        self.requests.lock().unwrap().push(request);
        Ok(BuiltResourceVariant {
            blob: BlobWriteIntent::Bytes(vec![7; 7]),
        })
    }
}

pub fn assert_release_outcome(
    outcome: ReleaseOutcome,
    remaining_owner_links: usize,
    delete_pending: bool,
) {
    assert_eq!(outcome.remaining_owner_links, remaining_owner_links);
    assert_eq!(outcome.delete_pending, delete_pending);
}
