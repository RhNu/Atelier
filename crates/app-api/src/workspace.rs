use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWorkspaceRequestDto {
    pub root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceStatusDto {
    pub root: PathBuf,
    pub schema_version: u32,
    pub locked: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseWorkspaceResponseDto {
    pub was_open: bool,
}
