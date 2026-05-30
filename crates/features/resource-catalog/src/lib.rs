//! Workspace resource catalog feature.

mod catalog;
mod error;
mod model;
mod ports;

pub use catalog::ResourceCatalog;
pub use error::{ResourceCatalogError, ResourceCatalogErrorKind, ResourceResult};
pub use model::{
    BlobId, BlobWriteIntent, CreateVariantRequest, RegisterResourceRequest, ReleaseOutcome,
    RepairReport, ResourceCleanupCandidate, ResourceCleanupReport, ResourceId, ResourceKind,
    ResourceLifecycle, ResourceLink, ResourceMetadata, ResourceOwner, ResourceOwnerKind,
    ResourceRecord, ResourceRef, ResourceRelation, ResourceState, ResourceVariant,
    ResourceVariantKind, StagedBlob, StagedBlobToken, VariantId,
};
pub use ports::{
    BuildVariantRequest, BuiltResourceVariant, ResourceBlobStore, ResourceCatalogRepository,
    ResourceCatalogTransaction, ResourceVariantBuilder,
};
