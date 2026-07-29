use super::{
    BlobId, Deserialize, Digest, Error, OpenOptions, Path, ResourceCatalogError, ResourceMetadata,
    ResourceResult, Serialize, Sha256, StagedBlobToken, SystemTime, UNIX_EPOCH, WORKSPACE_FORMAT,
    WORKSPACE_SCHEMA_VERSION, WorkspaceError, WorkspaceRelativePath, WorkspaceResult,
    WorkspaceSlot, Write, fs, io,
};
use std::sync::atomic::{AtomicU64, Ordering};

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredManifest {
    pub(super) format: String,
    pub(super) schema_version: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UntrustedStoredManifest {
    pub(super) format: Option<String>,
    pub(super) schema_version: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StagedBlobSidecar {
    pub(super) blob_id: String,
    pub(super) byte_size: Option<u64>,
    pub(super) content_hash: Option<String>,
}

impl StagedBlobSidecar {
    pub(super) fn from_blob(blob_id: &BlobId, metadata: &ResourceMetadata) -> Self {
        Self {
            blob_id: blob_id.as_str().to_owned(),
            byte_size: metadata.byte_size,
            content_hash: metadata.content_hash.clone(),
        }
    }
}

#[derive(Debug, Error)]
#[error("failed to {operation} `{path}`: {source}")]
struct FsOperationError {
    operation: &'static str,
    path: String,
    source: io::Error,
}

pub fn create_dir_all(path: &Path) -> WorkspaceResult<()> {
    fs::create_dir_all(path).map_err(|source| workspace_fs_error(path, source))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> WorkspaceResult<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|source| WorkspaceError::storage(source.to_string()))?;
    fs::write(path, text).map_err(|source| workspace_fs_error(path, source))
}

pub fn write_json_resource_exclusive(path: &Path, value: &impl Serialize) -> ResourceResult<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|source| ResourceCatalogError::blob_store(source.to_string()))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| resource_fs_error(path, source))?;
    file.write_all(text.as_bytes())
        .map_err(|source| resource_fs_error(path, source))?;
    file.write_all(b"\n")
        .map_err(|source| resource_fs_error(path, source))?;
    file.flush()
        .map_err(|source| resource_fs_error(path, source))
}

pub fn read_sidecar(path: &Path) -> ResourceResult<StagedBlobSidecar> {
    let text = fs::read_to_string(path).map_err(|source| resource_fs_error(path, source))?;
    serde_json::from_str(&text)
        .map_err(|source| ResourceCatalogError::blob_store(source.to_string()))
}

pub fn write_staging_part(file: &mut fs::File, path: &Path, bytes: &[u8]) -> ResourceResult<()> {
    file.write_all(bytes)
        .map_err(|source| resource_fs_error(path, source))?;
    file.flush()
        .map_err(|source| resource_fs_error(path, source))
}

pub fn validate_staged_part_matches_sidecar(
    part_path: &Path,
    sidecar: &StagedBlobSidecar,
) -> ResourceResult<()> {
    let blob_id = BlobId::new(sidecar.blob_id.clone());
    let blob_id_hash = parse_sha256_blob_id(&blob_id)?;
    if sidecar.content_hash.as_deref() != Some(blob_id_hash) {
        return Err(ResourceCatalogError::blob_store(
            "staged sidecar blob id does not match content hash",
        ));
    }
    let bytes = fs::read(part_path).map_err(|source| resource_fs_error(part_path, source))?;
    let actual_hash = sha256_hex(&bytes);
    if sidecar.content_hash.as_deref() != Some(actual_hash.as_str()) {
        return Err(ResourceCatalogError::blob_store(
            "staged blob content does not match staged sidecar hash",
        ));
    }
    if sidecar.byte_size != Some(bytes.len() as u64) {
        return Err(ResourceCatalogError::blob_store(
            "staged blob content does not match staged sidecar byte size",
        ));
    }
    Ok(())
}

pub fn remove_file_if_exists(path: &Path) -> ResourceResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(resource_fs_error(path, source)),
    }
}

pub fn workspace_fs_error(path: &Path, source: io::Error) -> WorkspaceError {
    WorkspaceError::storage(
        FsOperationError {
            operation: "access",
            path: path.display().to_string(),
            source,
        }
        .to_string(),
    )
}

pub fn resource_fs_error(path: &Path, source: io::Error) -> ResourceCatalogError {
    ResourceCatalogError::blob_store(
        FsOperationError {
            operation: "access",
            path: path.display().to_string(),
            source,
        }
        .to_string(),
    )
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub fn storage_path_for(slot: WorkspaceSlot) -> WorkspaceRelativePath {
    let value = match slot {
        WorkspaceSlot::ManifestFile => "workspace.json",
        WorkspaceSlot::LockFile => "locks/workspace.lock",
        WorkspaceSlot::ResourceBlobs => "resources/blobs",
        WorkspaceSlot::ResourceStaging => "resources/staging",
        WorkspaceSlot::ResourceVariants => "resources/variants",
        WorkspaceSlot::Database => "database",
        WorkspaceSlot::Cache => "cache",
        WorkspaceSlot::Exports => "exports",
    };
    WorkspaceRelativePath::new(value).expect("storage-fs built-in path should be valid")
}

pub fn parse_sha256_blob_id(blob_id: &BlobId) -> ResourceResult<&str> {
    let Some(hash) = blob_id.as_str().strip_prefix("sha256:") else {
        return Err(ResourceCatalogError::blob_store(
            "blob id must start with sha256:",
        ));
    };
    if hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(hash)
    } else {
        Err(ResourceCatalogError::blob_store(
            "blob id must contain a 64 character sha256 hash",
        ))
    }
}

pub fn validate_token(token: &StagedBlobToken) -> ResourceResult<()> {
    let value = token.as_str();
    if !value.is_empty() && !value.contains(['/', '\\', '\0']) {
        Ok(())
    } else {
        Err(ResourceCatalogError::blob_store(
            "staged blob token must be a file name",
        ))
    }
}

pub fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

pub fn unique_staged_blob_token() -> StagedBlobToken {
    StagedBlobToken::new(format!(
        "staged-{}-{}-{}",
        std::process::id(),
        unix_nanos(),
        STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

const _: () = {
    assert!(!WORKSPACE_FORMAT.is_empty());
    assert!(WORKSPACE_SCHEMA_VERSION == 1);
};
