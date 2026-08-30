use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DownloadableResourceStateDto {
    Missing,
    Downloading,
    Verifying,
    Ready,
    UpdateAvailable,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DownloadableResourceStatusDto {
    pub id: String,
    pub available_version: String,
    pub installed_version: Option<String>,
    pub state: DownloadableResourceStateDto,
    pub size_bytes: u64,
    pub downloaded_bytes: u64,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DownloadableResourceGroupDto {
    pub id: String,
    pub resources: Vec<String>,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DownloadableResourcesDto {
    pub catalog_version: String,
    pub onboarding_complete: bool,
    pub resources: Vec<DownloadableResourceStatusDto>,
    pub groups: Vec<DownloadableResourceGroupDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DownloadableResourceRequestDto {
    pub resource_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DownloadableResourceGroupRequestDto {
    pub group_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DownloadableResourceInstallProgressDto {
    pub resource_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}
