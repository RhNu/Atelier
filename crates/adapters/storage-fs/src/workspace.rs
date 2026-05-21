use super::{
    OpenOptions, PathBuf, StoredLockMetadata, StoredManifest, WorkspaceError, WorkspaceLayout,
    WorkspaceLock, WorkspaceLockLease, WorkspaceLockMetadata, WorkspaceLockRequest,
    WorkspaceManifest, WorkspaceRelativePath, WorkspaceResult, WorkspaceRoot, WorkspaceSlot,
    WorkspaceStore, async_trait, create_dir_all, fs, io, lock_file_token_matches, storage_path_for,
    unique_lock_token, unix_ms, workspace_fs_error, write_json, write_lock_metadata,
};

#[must_use]
pub fn workspace_relative_path_for(slot: WorkspaceSlot) -> WorkspaceRelativePath {
    storage_path_for(slot)
}

#[must_use]
pub fn workspace_slot_path(root: &WorkspaceRoot, slot: WorkspaceSlot) -> PathBuf {
    root.join_relative(&storage_path_for(slot))
}

#[must_use]
pub fn workspace_database_path(root: &WorkspaceRoot) -> PathBuf {
    workspace_slot_path(root, WorkspaceSlot::Database).join("atelier.sqlite3")
}

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
