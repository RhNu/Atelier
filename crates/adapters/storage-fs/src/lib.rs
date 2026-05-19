//! Filesystem storage adapters for NAI Atelier.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use nai_atelier_resource_catalog::{
    BlobId, BlobWriteIntent, ResourceBlobStore, ResourceCatalogError, ResourceMetadata,
    ResourceResult, StagedBlob, StagedBlobToken,
};
use nai_atelier_workspace::{
    WORKSPACE_SCHEMA_VERSION, WorkspaceError, WorkspaceLayout, WorkspaceLock, WorkspaceLockLease,
    WorkspaceLockMetadata, WorkspaceLockRequest, WorkspaceManifest, WorkspaceRelativePath,
    WorkspaceResult, WorkspaceRoot, WorkspaceSlot, WorkspaceStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);
static LOCK_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Copy, Clone, Debug, Default)]
pub struct FileSystemWorkspaceStore;

impl FileSystemWorkspaceStore {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkspaceStore for FileSystemWorkspaceStore {
    async fn initialize(
        &self,
        root: &WorkspaceRoot,
        layout: &WorkspaceLayout,
    ) -> WorkspaceResult<WorkspaceManifest> {
        create_dir_all(root.as_path())?;
        for slot in layout.directory_slots() {
            create_dir_all(&root.join_relative(&storage_path_for(*slot)))?;
        }
        if let Some(parent) = root
            .join_relative(&storage_path_for(WorkspaceSlot::LockFile))
            .parent()
        {
            create_dir_all(parent)?;
        }

        let manifest_path = root.join_relative(&storage_path_for(WorkspaceSlot::ManifestFile));
        if manifest_path.exists() {
            return self.load_manifest(root, layout).await;
        }
        let manifest = WorkspaceManifest::default();
        let stored = StoredManifest {
            schema_version: manifest.schema_version,
        };
        write_json(&manifest_path, &stored)?;
        Ok(manifest)
    }

    async fn load_manifest(
        &self,
        root: &WorkspaceRoot,
        _layout: &WorkspaceLayout,
    ) -> WorkspaceResult<WorkspaceManifest> {
        let path = root.join_relative(&storage_path_for(WorkspaceSlot::ManifestFile));
        let text = fs::read_to_string(&path).map_err(|source| workspace_fs_error(&path, source))?;
        let stored: StoredManifest = serde_json::from_str(&text)
            .map_err(|source| WorkspaceError::storage(source.to_string()))?;
        WorkspaceManifest {
            schema_version: stored.schema_version,
        }
        .validate()
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct FileSystemWorkspaceLock;

impl FileSystemWorkspaceLock {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait]
impl WorkspaceLock for FileSystemWorkspaceLock {
    async fn acquire(
        &self,
        root: &WorkspaceRoot,
        _layout: &WorkspaceLayout,
        request: WorkspaceLockRequest,
    ) -> WorkspaceResult<Box<dyn WorkspaceLockLease>> {
        let path = root.join_relative(&storage_path_for(WorkspaceSlot::LockFile));
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == io::ErrorKind::AlreadyExists {
                    WorkspaceError::locked(format!("workspace is locked by `{}`", path.display()))
                } else {
                    workspace_fs_error(&path, source)
                }
            })?;
        let metadata = WorkspaceLockMetadata::new(request.holder, unix_ms());
        let token = unique_lock_token();
        let stored = StoredLockMetadata::from_metadata(&metadata, token.clone());
        if let Err(error) = write_lock_metadata(&mut file, &path, &stored) {
            let _remove_result = fs::remove_file(&path);
            return Err(error);
        }
        Ok(Box::new(FileSystemWorkspaceLockLease {
            path,
            metadata,
            token,
        }))
    }
}

#[derive(Debug)]
struct FileSystemWorkspaceLockLease {
    path: PathBuf,
    metadata: WorkspaceLockMetadata,
    token: String,
}

#[async_trait]
impl WorkspaceLockLease for FileSystemWorkspaceLockLease {
    async fn metadata(&self) -> WorkspaceResult<WorkspaceLockMetadata> {
        Ok(self.metadata.clone())
    }
}

impl Drop for FileSystemWorkspaceLockLease {
    fn drop(&mut self) {
        if lock_file_token_matches(&self.path, &self.token) {
            let _remove_result = fs::remove_file(&self.path);
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileSystemResourceBlobStore {
    root: WorkspaceRoot,
}

impl FileSystemResourceBlobStore {
    #[must_use]
    pub const fn new(root: WorkspaceRoot, _layout: WorkspaceLayout) -> Self {
        Self { root }
    }

    /// Returns the final filesystem path for a catalog blob id.
    ///
    /// # Errors
    /// Returns an error when the blob id is not a storage-fs sha256 blob id.
    pub fn blob_path(&self, blob_id: &BlobId) -> ResourceResult<PathBuf> {
        let hash = parse_sha256_blob_id(blob_id)?;
        let prefix = hash
            .get(..2)
            .ok_or_else(|| ResourceCatalogError::blob_store("sha256 hash is too short"))?;
        Ok(self
            .root
            .join_relative(&storage_path_for(WorkspaceSlot::ResourceBlobs))
            .join("sha256")
            .join(prefix)
            .join(hash))
    }

    fn staging_part_path(&self, token: &StagedBlobToken) -> ResourceResult<PathBuf> {
        self.staging_path(token, "part")
    }

    fn staging_sidecar_path(&self, token: &StagedBlobToken) -> ResourceResult<PathBuf> {
        self.staging_path(token, "json")
    }

    fn staging_path(&self, token: &StagedBlobToken, extension: &str) -> ResourceResult<PathBuf> {
        validate_token(token)?;
        Ok(self
            .root
            .join_relative(&storage_path_for(WorkspaceSlot::ResourceStaging))
            .join(format!("{}.{}", token.as_str(), extension)))
    }
}

#[async_trait]
impl ResourceBlobStore for FileSystemResourceBlobStore {
    async fn stage_blob(&self, intent: BlobWriteIntent) -> ResourceResult<StagedBlob> {
        let BlobWriteIntent::Bytes(bytes) = intent;
        let hash = sha256_hex(&bytes);
        let blob_id = BlobId::new(format!("sha256:{hash}"));
        let metadata = ResourceMetadata {
            byte_size: Some(bytes.len() as u64),
            content_hash: Some(hash),
            ..ResourceMetadata::default()
        };
        let (token, part_path) = self.create_unique_staging_part(&bytes)?;
        let sidecar_path = self.staging_sidecar_path(&token)?;
        let sidecar = StagedBlobSidecar::from_blob(&blob_id, &metadata);
        if let Err(error) = write_json_resource_exclusive(&sidecar_path, &sidecar) {
            let _remove_result = fs::remove_file(&part_path);
            return Err(error);
        }
        Ok(StagedBlob {
            token,
            blob_id,
            metadata,
        })
    }

    async fn finalize_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        let part_path = self.staging_part_path(staged)?;
        let sidecar_path = self.staging_sidecar_path(staged)?;
        let sidecar = read_sidecar(&sidecar_path)?;
        let blob_id = BlobId::new(sidecar.blob_id.clone());
        let blob_path = self.blob_path(&blob_id)?;
        validate_staged_part_matches_sidecar(&part_path, &sidecar)?;
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).map_err(|source| resource_fs_error(parent, source))?;
        }
        if blob_path.exists() {
            fs::remove_file(&part_path).map_err(|source| resource_fs_error(&part_path, source))?;
        } else {
            fs::rename(&part_path, &blob_path)
                .map_err(|source| resource_fs_error(&blob_path, source))?;
        }
        remove_file_if_exists(&sidecar_path)?;
        Ok(())
    }

    async fn abort_staged_blob(&self, staged: &StagedBlobToken) -> ResourceResult<()> {
        remove_file_if_exists(&self.staging_part_path(staged)?)?;
        remove_file_if_exists(&self.staging_sidecar_path(staged)?)?;
        Ok(())
    }

    async fn delete_blob(&self, blob_id: &BlobId) -> ResourceResult<()> {
        remove_file_if_exists(&self.blob_path(blob_id)?)
    }

    async fn blob_exists(&self, blob_id: &BlobId) -> ResourceResult<bool> {
        Ok(self.blob_path(blob_id)?.exists())
    }
}

impl FileSystemResourceBlobStore {
    fn create_unique_staging_part(
        &self,
        bytes: &[u8],
    ) -> ResourceResult<(StagedBlobToken, PathBuf)> {
        for _attempt in 0..32 {
            let token = unique_staged_blob_token();
            let part_path = self.staging_part_path(&token)?;
            if let Some(parent) = part_path.parent() {
                fs::create_dir_all(parent).map_err(|source| resource_fs_error(parent, source))?;
            }
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&part_path)
            {
                Ok(mut file) => {
                    if let Err(error) = write_staging_part(&mut file, &part_path, bytes) {
                        let _remove_result = fs::remove_file(&part_path);
                        return Err(error);
                    }
                    return Ok((token, part_path));
                }
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {}
                Err(source) => return Err(resource_fs_error(&part_path, source)),
            }
        }
        Err(ResourceCatalogError::blob_store(
            "failed to create a unique staged blob token",
        ))
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct StoredManifest {
    schema_version: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredLockMetadata {
    token: String,
    holder: String,
    created_at_ms: u64,
}

impl StoredLockMetadata {
    fn from_metadata(metadata: &WorkspaceLockMetadata, token: String) -> Self {
        Self {
            token,
            holder: metadata.holder.clone(),
            created_at_ms: metadata.created_at_ms,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StagedBlobSidecar {
    blob_id: String,
    byte_size: Option<u64>,
    content_hash: Option<String>,
}

impl StagedBlobSidecar {
    fn from_blob(blob_id: &BlobId, metadata: &ResourceMetadata) -> Self {
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

fn create_dir_all(path: &Path) -> WorkspaceResult<()> {
    fs::create_dir_all(path).map_err(|source| workspace_fs_error(path, source))
}

fn write_json(path: &Path, value: &impl Serialize) -> WorkspaceResult<()> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|source| WorkspaceError::storage(source.to_string()))?;
    fs::write(path, text).map_err(|source| workspace_fs_error(path, source))
}

fn write_json_resource_exclusive(path: &Path, value: &impl Serialize) -> ResourceResult<()> {
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

fn read_sidecar(path: &Path) -> ResourceResult<StagedBlobSidecar> {
    let text = fs::read_to_string(path).map_err(|source| resource_fs_error(path, source))?;
    serde_json::from_str(&text)
        .map_err(|source| ResourceCatalogError::blob_store(source.to_string()))
}

fn write_lock_metadata(
    file: &mut fs::File,
    path: &Path,
    metadata: &StoredLockMetadata,
) -> WorkspaceResult<()> {
    serde_json::to_writer(&mut *file, metadata)
        .map_err(|source| WorkspaceError::storage(source.to_string()))?;
    file.write_all(b"\n")
        .map_err(|source| workspace_fs_error(path, source))?;
    file.flush()
        .map_err(|source| workspace_fs_error(path, source))
}

fn write_staging_part(file: &mut fs::File, path: &Path, bytes: &[u8]) -> ResourceResult<()> {
    file.write_all(bytes)
        .map_err(|source| resource_fs_error(path, source))?;
    file.flush()
        .map_err(|source| resource_fs_error(path, source))
}

fn lock_file_token_matches(path: &Path, expected: &str) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(metadata) = serde_json::from_str::<StoredLockMetadata>(&text) else {
        return false;
    };
    metadata.token == expected
}

fn validate_staged_part_matches_sidecar(
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

fn remove_file_if_exists(path: &Path) -> ResourceResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(resource_fs_error(path, source)),
    }
}

fn workspace_fs_error(path: &Path, source: io::Error) -> WorkspaceError {
    WorkspaceError::storage(
        FsOperationError {
            operation: "access",
            path: path.display().to_string(),
            source,
        }
        .to_string(),
    )
}

fn resource_fs_error(path: &Path, source: io::Error) -> ResourceCatalogError {
    ResourceCatalogError::blob_store(
        FsOperationError {
            operation: "access",
            path: path.display().to_string(),
            source,
        }
        .to_string(),
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn storage_path_for(slot: WorkspaceSlot) -> WorkspaceRelativePath {
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

fn parse_sha256_blob_id(blob_id: &BlobId) -> ResourceResult<&str> {
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

fn validate_token(token: &StagedBlobToken) -> ResourceResult<()> {
    let value = token.as_str();
    if !value.is_empty() && !value.contains(['/', '\\', '\0']) {
        Ok(())
    } else {
        Err(ResourceCatalogError::blob_store(
            "staged blob token must be a file name",
        ))
    }
}

fn unix_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn unique_staged_blob_token() -> StagedBlobToken {
    StagedBlobToken::new(format!(
        "staged-{}-{}-{}",
        std::process::id(),
        unix_nanos(),
        STAGING_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn unique_lock_token() -> String {
    format!(
        "lock-{}-{}-{}",
        std::process::id(),
        unix_nanos(),
        LOCK_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

const _: () = {
    assert!(WORKSPACE_SCHEMA_VERSION == 1);
};
