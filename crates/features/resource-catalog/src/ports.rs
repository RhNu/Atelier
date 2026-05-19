use async_trait::async_trait;

use crate::model::{
    BlobId, BlobWriteIntent, ResourceId, ResourceLink, ResourceRecord, ResourceRef,
    ResourceVariant, ResourceVariantKind, StagedBlob, StagedBlobToken, VariantId,
};
use crate::{ResourceOwner, ResourceResult};

#[async_trait]
pub trait ResourceCatalogRepository: Send + Sync {
    async fn begin_transaction(&self) -> ResourceResult<Box<dyn ResourceCatalogTransaction>>;

    async fn get_ready_record(&self, id: &ResourceId) -> ResourceResult<Option<ResourceRecord>>;

    async fn list_ready_refs_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceRef>>;

    async fn list_ready_links_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceLink>>;

    async fn get_variant(&self, id: &VariantId) -> ResourceResult<Option<ResourceVariant>>;

    async fn scan_orphan_blobs(&self) -> ResourceResult<Vec<BlobId>>;

    async fn record_orphan_blob(&self, blob_id: &BlobId) -> ResourceResult<()>;
}

#[async_trait]
pub trait ResourceCatalogTransaction: Send {
    async fn insert_pending_record(&mut self, record: ResourceRecord) -> ResourceResult<()>;

    async fn attach_owner(&mut self, link: ResourceLink) -> ResourceResult<()>;

    async fn detach_owner(&mut self, link: &ResourceLink) -> ResourceResult<()>;

    async fn count_owner_links(&self, id: &ResourceId) -> ResourceResult<usize>;

    async fn mark_ready(&mut self, id: &ResourceId) -> ResourceResult<()>;

    async fn mark_delete_pending(&mut self, id: &ResourceId) -> ResourceResult<()>;

    async fn insert_variant(&mut self, variant: ResourceVariant) -> ResourceResult<()>;

    async fn clear_orphan_blob_marker(&mut self, blob_id: &BlobId) -> ResourceResult<()>;

    async fn commit(self: Box<Self>) -> ResourceResult<()>;

    async fn rollback(self: Box<Self>) -> ResourceResult<()>;
}

#[async_trait]
pub trait ResourceBlobStore: Send + Sync {
    async fn stage_blob(&self, intent: BlobWriteIntent) -> ResourceResult<StagedBlob>;

    async fn finalize_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()>;

    async fn abort_staged_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()>;

    async fn delete_blob(&self, blob_id: &BlobId) -> ResourceResult<()>;

    async fn blob_exists(&self, blob_id: &BlobId) -> ResourceResult<bool>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildVariantRequest {
    pub source: ResourceRef,
    pub source_record: ResourceRecord,
    pub variant_id: VariantId,
    pub kind: ResourceVariantKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuiltResourceVariant {
    pub blob: BlobWriteIntent,
}

#[async_trait]
pub trait ResourceVariantBuilder: Send + Sync {
    async fn build_variant(
        &self,
        request: BuildVariantRequest,
    ) -> ResourceResult<BuiltResourceVariant>;
}
