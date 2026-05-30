#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlobId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariantId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StagedBlobToken(String);

macro_rules! opaque_id {
    ($type_name:ident) => {
        impl $type_name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_id!(ResourceId);
opaque_id!(BlobId);
opaque_id!(VariantId);
opaque_id!(StagedBlobToken);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    GeneratedImage,
    StreamFinalImage,
    DirectorResult,
    SourceImage,
    ReferenceImage,
    ControlNetImage,
    PromptThumb,
    VibeDocument,
    VibePreview,
    VibeEncoding,
    LexiconBundle,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceLifecycle {
    WorkspaceScoped,
    JobScoped,
    Cache,
    ExportOnly,
}

impl ResourceLifecycle {
    #[must_use]
    pub const fn is_auto_releasable(self) -> bool {
        matches!(self, Self::JobScoped | Self::Cache)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceState {
    Pending,
    Ready,
    DeletePending,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceOwnerKind {
    Job,
    GalleryItem,
    PromptResource,
    Vibe,
    DirectorRun,
    Cache,
    ImportStaging,
    Workspace,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceOwner {
    pub kind: ResourceOwnerKind,
    pub local_id: String,
}

impl ResourceOwner {
    #[must_use]
    pub fn new(kind: ResourceOwnerKind, local_id: impl Into<String>) -> Self {
        Self {
            kind,
            local_id: local_id.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceRelation {
    Primary,
    Source,
    Reference,
    Thumbnail,
    Preview,
    Encoding,
    DerivedFrom,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceLink {
    pub resource_id: ResourceId,
    pub owner: ResourceOwner,
    pub relation: ResourceRelation,
}

impl ResourceLink {
    #[must_use]
    pub const fn new(
        resource_id: ResourceId,
        owner: ResourceOwner,
        relation: ResourceRelation,
    ) -> Self {
        Self {
            resource_id,
            owner,
            relation,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceMetadata {
    pub mime_type: Option<String>,
    pub byte_size: Option<u64>,
    pub content_hash: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRecord {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub lifecycle: ResourceLifecycle,
    pub state: ResourceState,
    pub blob_id: BlobId,
    pub metadata: ResourceMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRef {
    pub id: ResourceId,
    pub variant_id: Option<VariantId>,
}

impl ResourceRef {
    #[must_use]
    pub const fn new(id: ResourceId, variant_id: Option<VariantId>) -> Self {
        Self { id, variant_id }
    }

    #[must_use]
    pub const fn base(id: ResourceId) -> Self {
        Self::new(id, None)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceVariantKind {
    Original,
    Preview,
    Thumbnail,
    Sanitized,
    Export,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceVariant {
    pub id: VariantId,
    pub resource_id: ResourceId,
    pub kind: ResourceVariantKind,
    pub blob_id: BlobId,
    pub metadata: ResourceMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedBlob {
    pub token: StagedBlobToken,
    pub blob_id: BlobId,
    pub metadata: ResourceMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlobWriteIntent {
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterResourceRequest {
    pub resource_id: ResourceId,
    pub kind: ResourceKind,
    pub lifecycle: ResourceLifecycle,
    pub owner: ResourceOwner,
    pub relation: ResourceRelation,
    pub blob: BlobWriteIntent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateVariantRequest {
    pub source: ResourceRef,
    pub variant_id: VariantId,
    pub kind: ResourceVariantKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ReleaseOutcome {
    pub remaining_owner_links: usize,
    pub delete_pending: bool,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RepairReport {
    pub deleted_orphan_blobs: usize,
    pub cleared_orphan_markers: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCleanupCandidate {
    pub record: ResourceRecord,
    pub variants: Vec<ResourceVariant>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct ResourceCleanupReport {
    pub resources_deleted: usize,
    pub blobs_deleted: usize,
}
