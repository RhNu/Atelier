use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CloseWorkspaceResponseDto {
    pub was_open: bool,
}
