use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{error::ErrorEnvelopeDto, settings::GlobalSettingsDto};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OpenWorkspaceRequestDto {
    pub root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WorkspaceStatusDto {
    pub root: PathBuf,
    pub schema_version: u32,
    pub locked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WorkspaceRestoreFailureDto {
    pub root: PathBuf,
    pub error: ErrorEnvelopeDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct AppBootstrapDto {
    pub global_settings: GlobalSettingsDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceStatusDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_failure: Option<WorkspaceRestoreFailureDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CloseWorkspaceResponseDto {
    pub was_open: bool,
}
