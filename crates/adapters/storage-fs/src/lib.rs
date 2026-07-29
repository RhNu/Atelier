//! Filesystem storage adapters for Atelier.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use atelier_resource_catalog::{
    BlobId, BlobWriteIntent, ResourceBlobStore, ResourceCatalogError, ResourceCatalogRepository,
    ResourceKind, ResourceMetadata, ResourceRef, ResourceResult, StagedBlob, StagedBlobToken,
};
use atelier_workspace::{
    WORKSPACE_FORMAT, WORKSPACE_SCHEMA_VERSION, WorkspaceError, WorkspaceLayout, WorkspaceLock,
    WorkspaceLockLease, WorkspaceManifest, WorkspaceRelativePath, WorkspaceResult, WorkspaceRoot,
    WorkspaceSlot, WorkspaceStore,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

mod resource;
mod support;
mod workspace;

use support::{
    StagedBlobSidecar, StoredManifest, UntrustedStoredManifest, create_dir_all,
    parse_sha256_blob_id, read_sidecar, remove_file_if_exists, resource_fs_error, sha256_hex,
    storage_path_for, unique_staged_blob_token, validate_staged_part_matches_sidecar,
    validate_token, workspace_fs_error, write_json, write_json_resource_exclusive,
    write_staging_part,
};

pub use resource::{FileSystemResourceBlobStore, FileSystemResourceContentReader, ResourceContent};
pub use workspace::{
    FileSystemWorkspaceLock, FileSystemWorkspaceStore, workspace_database_path,
    workspace_relative_path_for, workspace_slot_path,
};
