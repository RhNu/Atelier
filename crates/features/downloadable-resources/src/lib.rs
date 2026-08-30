//! Global, reconstructable runtime resource contracts.

mod catalog;
mod error;
mod model;
mod ports;

pub use catalog::validate_catalog;
pub use error::{DownloadableResourceError, DownloadableResourceResult};
pub use model::{
    DownloadableResourceCatalog, DownloadableResourceDescriptor, DownloadableResourceFile,
    DownloadableResourceGroup, DownloadableResourceState, DownloadableResourceStatus,
    InstalledResource, ResourceInstallProgress,
};
pub use ports::{DownloadableResourceManager, ResourceInstallProgressSink};

pub const CATALOG_FORMAT: &str = "atelier.downloadable-resource-catalog";
pub const CATALOG_SCHEMA_VERSION: u32 = 1;
