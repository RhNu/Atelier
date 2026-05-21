use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_adapter_storage_fs::{
    FileSystemResourceBlobStore, FileSystemWorkspaceLock, FileSystemWorkspaceStore,
};
use atelier_resource_catalog::{
    BlobId, BlobWriteIntent, BuildVariantRequest, BuiltResourceVariant, ResourceBlobStore,
    ResourceCatalog, ResourceCatalogError, ResourceCatalogRepository, ResourceCatalogTransaction,
    ResourceId, ResourceKind, ResourceLifecycle, ResourceOwner, ResourceOwnerKind, ResourceRecord,
    ResourceRef, ResourceRelation, ResourceState, ResourceVariant, ResourceVariantBuilder,
    VariantId,
};
use atelier_workspace::{
    WorkspaceErrorKind, WorkspaceLayout, WorkspaceLock, WorkspaceLockRequest,
    WorkspaceRelativePath, WorkspaceRoot, WorkspaceSlot, WorkspaceStore,
};
use futures_executor::block_on;

#[test]
fn initialize_creates_workspace_manifest_and_internal_directories() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let root = WorkspaceRoot::new(temp.path().join("atelier"));
        let layout = WorkspaceLayout;

        let manifest = FileSystemWorkspaceStore::new()
            .initialize(&root, &layout)
            .await
            .unwrap();

        assert_eq!(manifest.schema_version, 1);
        assert!(
            root.join_relative(&storage_path(WorkspaceSlot::ManifestFile))
                .exists()
        );
        assert!(
            root.join_relative(&storage_path(WorkspaceSlot::ResourceBlobs))
                .is_dir()
        );
        assert!(
            root.join_relative(&storage_path(WorkspaceSlot::ResourceStaging))
                .is_dir()
        );
        assert!(
            root.join_relative(&storage_path(WorkspaceSlot::ResourceVariants))
                .is_dir()
        );
        assert!(
            root.join_relative(&storage_path(WorkspaceSlot::LockFile))
                .parent()
                .unwrap()
                .is_dir()
        );
    });
}

#[test]
fn workspace_lock_rejects_second_holder_until_first_lease_drops() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let root = WorkspaceRoot::new(temp.path());
        let layout = WorkspaceLayout;
        FileSystemWorkspaceStore::new()
            .initialize(&root, &layout)
            .await
            .unwrap();
        let lock = FileSystemWorkspaceLock::new();

        let first = lock
            .acquire(&root, &layout, WorkspaceLockRequest::new("first"))
            .await
            .unwrap();
        let error = lock
            .acquire(&root, &layout, WorkspaceLockRequest::new("second"))
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind, WorkspaceErrorKind::Locked);

        drop(first);
        let second = lock
            .acquire(&root, &layout, WorkspaceLockRequest::new("second"))
            .await
            .unwrap();
        let metadata = second.metadata().await.unwrap();
        assert_eq!(metadata.holder, "second");
    });
}

#[test]
fn dropping_old_lock_lease_does_not_remove_newer_lock() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let root = WorkspaceRoot::new(temp.path());
        let layout = WorkspaceLayout;
        FileSystemWorkspaceStore::new()
            .initialize(&root, &layout)
            .await
            .unwrap();
        let lock = FileSystemWorkspaceLock::new();

        let first = lock
            .acquire(&root, &layout, WorkspaceLockRequest::new("first"))
            .await
            .unwrap();
        std::fs::remove_file(root.join_relative(&storage_path(WorkspaceSlot::LockFile))).unwrap();
        let second = lock
            .acquire(&root, &layout, WorkspaceLockRequest::new("second"))
            .await
            .unwrap();

        drop(first);
        let error = lock
            .acquire(&root, &layout, WorkspaceLockRequest::new("third"))
            .await
            .err()
            .unwrap();

        assert_eq!(error.kind, WorkspaceErrorKind::Locked);
        drop(second);
    });
}

#[test]
fn blob_stage_finalize_hashes_bytes_and_cleans_staging() {
    block_on(async {
        let (root, _layout, store) = initialized_blob_store().await;

        let staged = store
            .stage_blob(BlobWriteIntent::Bytes(b"abc".to_vec()))
            .await
            .unwrap();
        assert_eq!(staged.metadata.byte_size, Some(3));
        assert_eq!(
            staged.metadata.content_hash.as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );

        store.finalize_blob(&staged.token).await.unwrap();

        let blob_path = store.blob_path(&staged.blob_id).unwrap();
        assert_eq!(std::fs::read(blob_path).unwrap(), b"abc");
        assert!(store.blob_exists(&staged.blob_id).await.unwrap());
        assert!(staging_entries(&root).is_empty());
    });
}

#[test]
fn finalize_rejects_staged_part_that_does_not_match_sidecar_hash() {
    block_on(async {
        let (root, _layout, store) = initialized_blob_store().await;
        let staged = store
            .stage_blob(BlobWriteIntent::Bytes(b"abc".to_vec()))
            .await
            .unwrap();
        let part_path = root
            .join_relative(&storage_path(WorkspaceSlot::ResourceStaging))
            .join(format!("{}.part", staged.token.as_str()));
        std::fs::write(&part_path, b"not abc").unwrap();

        let error = store.finalize_blob(&staged.token).await.unwrap_err();

        assert!(error.to_string().contains("does not match staged sidecar"));
        assert!(!store.blob_exists(&staged.blob_id).await.unwrap());
        assert!(!staging_entries(&root).is_empty());
    });
}

#[test]
fn finalize_rejects_sidecar_blob_id_that_does_not_match_content_hash() {
    block_on(async {
        let (root, _layout, store) = initialized_blob_store().await;
        let staged = store
            .stage_blob(BlobWriteIntent::Bytes(b"abc".to_vec()))
            .await
            .unwrap();
        let sidecar_path = root
            .join_relative(&storage_path(WorkspaceSlot::ResourceStaging))
            .join(format!("{}.json", staged.token.as_str()));
        let mut sidecar: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&sidecar_path).unwrap()).unwrap();
        sidecar["blob_id"] = serde_json::Value::String(
            "sha256:0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
        );
        std::fs::write(
            &sidecar_path,
            serde_json::to_string_pretty(&sidecar).unwrap(),
        )
        .unwrap();

        let error = store.finalize_blob(&staged.token).await.unwrap_err();

        assert!(error.to_string().contains("blob id does not match"));
        assert!(
            !store
                .blob_exists(&BlobId::new(
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000",
                ))
                .await
                .unwrap()
        );
        assert!(!staging_entries(&root).is_empty());
    });
}

#[test]
fn abort_staged_blob_removes_part_and_sidecar() {
    block_on(async {
        let (root, _layout, store) = initialized_blob_store().await;
        let staged = store
            .stage_blob(BlobWriteIntent::Bytes(vec![1, 2, 3]))
            .await
            .unwrap();

        store.abort_staged_blob(&staged.token).await.unwrap();

        assert!(staging_entries(&root).is_empty());
        assert!(!store.blob_exists(&staged.blob_id).await.unwrap());
    });
}

#[test]
fn delete_blob_is_ok_when_blob_is_missing() {
    block_on(async {
        let (_, _, store) = initialized_blob_store().await;
        let missing =
            BlobId::new("sha256:0000000000000000000000000000000000000000000000000000000000000000");

        store.delete_blob(&missing).await.unwrap();
    });
}

#[test]
fn storage_blob_store_registers_resource_through_resource_catalog() {
    block_on(async {
        let (_, _, blob_store) = initialized_blob_store().await;
        let catalog =
            ResourceCatalog::new(SimpleRepository::default(), blob_store, NullVariantBuilder);
        let owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");

        let reference = catalog
            .register_resource(atelier_resource_catalog::RegisterResourceRequest {
                resource_id: ResourceId::new("generated-1"),
                kind: ResourceKind::GeneratedImage,
                lifecycle: ResourceLifecycle::JobScoped,
                owner,
                relation: ResourceRelation::Primary,
                blob: BlobWriteIntent::Bytes(vec![7, 8, 9]),
            })
            .await
            .unwrap();

        let record = catalog.get(&reference).await.unwrap();
        assert_eq!(reference, ResourceRef::base(ResourceId::new("generated-1")));
        assert_eq!(record.metadata.byte_size, Some(3));
        assert_eq!(record.state, ResourceState::Ready);
    });
}

async fn initialized_blob_store() -> (WorkspaceRoot, WorkspaceLayout, FileSystemResourceBlobStore) {
    let temp = tempfile::tempdir().unwrap().keep();
    let root = WorkspaceRoot::new(temp);
    let layout = WorkspaceLayout;
    FileSystemWorkspaceStore::new()
        .initialize(&root, &layout)
        .await
        .unwrap();
    let store = FileSystemResourceBlobStore::new(root.clone(), layout);
    (root, layout, store)
}

fn staging_entries(root: &WorkspaceRoot) -> Vec<String> {
    let path = root.join_relative(&storage_path(WorkspaceSlot::ResourceStaging));
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn storage_path(slot: WorkspaceSlot) -> WorkspaceRelativePath {
    let value = match slot {
        WorkspaceSlot::ManifestFile => "workspace.json",
        WorkspaceSlot::LockFile => "locks/workspace.lock",
        WorkspaceSlot::ResourceBlobs => "resources/blobs",
        WorkspaceSlot::ResourceStaging => "resources/staging",
        WorkspaceSlot::ResourceVariants => "resources/variants",
        WorkspaceSlot::Database => "database",
        WorkspaceSlot::Cache => "cache",
        WorkspaceSlot::Exports => "exports",
    };
    WorkspaceRelativePath::new(value).unwrap()
}

#[derive(Clone, Default)]
struct SimpleRepository {
    state: Arc<Mutex<RepositoryState>>,
}

#[derive(Default)]
struct RepositoryState {
    records: BTreeMap<ResourceId, ResourceRecord>,
}

#[async_trait]
impl ResourceCatalogRepository for SimpleRepository {
    async fn begin_transaction(
        &self,
    ) -> atelier_resource_catalog::ResourceResult<Box<dyn ResourceCatalogTransaction>> {
        Ok(Box::new(SimpleTransaction {
            state: Arc::clone(&self.state),
        }))
    }

    async fn get_ready_record(
        &self,
        id: &ResourceId,
    ) -> atelier_resource_catalog::ResourceResult<Option<ResourceRecord>> {
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
        _owner: &ResourceOwner,
    ) -> atelier_resource_catalog::ResourceResult<Vec<ResourceRef>> {
        Ok(Vec::new())
    }

    async fn list_ready_links_by_owner(
        &self,
        _owner: &ResourceOwner,
    ) -> atelier_resource_catalog::ResourceResult<Vec<atelier_resource_catalog::ResourceLink>> {
        Ok(Vec::new())
    }

    async fn get_variant(
        &self,
        _id: &VariantId,
    ) -> atelier_resource_catalog::ResourceResult<Option<ResourceVariant>> {
        Ok(None)
    }

    async fn scan_orphan_blobs(&self) -> atelier_resource_catalog::ResourceResult<Vec<BlobId>> {
        Ok(Vec::new())
    }

    async fn record_orphan_blob(
        &self,
        _blob_id: &BlobId,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }
}

struct SimpleTransaction {
    state: Arc<Mutex<RepositoryState>>,
}

#[async_trait]
impl ResourceCatalogTransaction for SimpleTransaction {
    async fn insert_pending_record(
        &mut self,
        record: ResourceRecord,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        self.state
            .lock()
            .unwrap()
            .records
            .insert(record.id.clone(), record);
        Ok(())
    }

    async fn attach_owner(
        &mut self,
        _link: atelier_resource_catalog::ResourceLink,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }

    async fn detach_owner(
        &mut self,
        _link: &atelier_resource_catalog::ResourceLink,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }

    async fn count_owner_links(
        &self,
        _id: &ResourceId,
    ) -> atelier_resource_catalog::ResourceResult<usize> {
        Ok(1)
    }

    async fn mark_ready(
        &mut self,
        id: &ResourceId,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        self.state
            .lock()
            .unwrap()
            .records
            .get_mut(id)
            .unwrap()
            .state = ResourceState::Ready;
        Ok(())
    }

    async fn mark_delete_pending(
        &mut self,
        _id: &ResourceId,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }

    async fn insert_variant(
        &mut self,
        _variant: ResourceVariant,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }

    async fn clear_orphan_blob_marker(
        &mut self,
        _blob_id: &BlobId,
    ) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }

    async fn commit(self: Box<Self>) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }

    async fn rollback(self: Box<Self>) -> atelier_resource_catalog::ResourceResult<()> {
        Ok(())
    }
}

struct NullVariantBuilder;

#[async_trait]
impl ResourceVariantBuilder for NullVariantBuilder {
    async fn build_variant(
        &self,
        _request: BuildVariantRequest,
    ) -> atelier_resource_catalog::ResourceResult<BuiltResourceVariant> {
        Err(ResourceCatalogError::variant_builder(
            "variant building is not used in this test",
        ))
    }
}
