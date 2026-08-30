use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DownloadableResourceCatalog {
    pub format: String,
    pub schema_version: u32,
    pub catalog_version: String,
    pub resources: Vec<DownloadableResourceDescriptor>,
    pub groups: Vec<DownloadableResourceGroup>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DownloadableResourceDescriptor {
    pub id: String,
    pub version: String,
    pub contract_version: u32,
    pub dependencies: Vec<String>,
    pub files: Vec<DownloadableResourceFile>,
}

impl DownloadableResourceDescriptor {
    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.files.iter().map(|file| file.size_bytes).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DownloadableResourceFile {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub urls: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DownloadableResourceGroup {
    pub id: String,
    pub resources: Vec<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DownloadableResourceState {
    Missing,
    Downloading,
    Verifying,
    Ready,
    UpdateAvailable,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadableResourceStatus {
    pub id: String,
    pub available_version: String,
    pub installed_version: Option<String>,
    pub state: DownloadableResourceState,
    pub size_bytes: u64,
    pub downloaded_bytes: u64,
    pub message: Option<String>,
}

#[derive(Clone)]
pub struct InstalledResource {
    pub id: String,
    pub version: String,
    pub root: PathBuf,
    pub lease: Arc<dyn Send + Sync>,
}

impl std::fmt::Debug for InstalledResource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InstalledResource")
            .field("id", &self.id)
            .field("version", &self.version)
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceInstallProgress {
    pub resource_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}
