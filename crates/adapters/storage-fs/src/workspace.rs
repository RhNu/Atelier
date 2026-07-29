use super::{
    OpenOptions, PathBuf, StoredManifest, UntrustedStoredManifest, WorkspaceError, WorkspaceLayout,
    WorkspaceLock, WorkspaceLockLease, WorkspaceManifest, WorkspaceRelativePath, WorkspaceResult,
    WorkspaceRoot, WorkspaceSlot, WorkspaceStore, async_trait, create_dir_all, fs,
    storage_path_for, workspace_fs_error, write_json,
};
use std::fs::TryLockError;

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
        let manifest_path = root.join_relative(&storage_path_for(WorkspaceSlot::ManifestFile));
        if manifest_path.exists() {
            return self.load_manifest(root, layout).await;
        }
        if root.as_path().exists() {
            let mut entries = fs::read_dir(root.as_path())
                .map_err(|source| workspace_fs_error(root.as_path(), source))?;
            if entries
                .next()
                .transpose()
                .map_err(|source| workspace_fs_error(root.as_path(), source))?
                .is_some()
            {
                return Err(WorkspaceError::unsupported_schema(format!(
                    "workspace directory `{}` is not empty and has no Atelier manifest",
                    root.as_path().display()
                )));
            }
        }

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

        let manifest = WorkspaceManifest::default();
        let stored = StoredManifest {
            format: manifest.format.clone(),
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
        let stored: UntrustedStoredManifest = serde_json::from_str(&text)
            .map_err(|source| WorkspaceError::storage(source.to_string()))?;
        WorkspaceManifest {
            format: stored.format.unwrap_or_else(|| "<missing>".to_owned()),
            schema_version: stored.schema_version.unwrap_or(0),
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
    ) -> WorkspaceResult<Box<dyn WorkspaceLockLease>> {
        let path = root.join_relative(&storage_path_for(WorkspaceSlot::LockFile));
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| workspace_fs_error(&path, source))?;
        match file.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(WorkspaceError::locked(format!(
                    "workspace is locked by `{}`",
                    path.display()
                )));
            }
            Err(TryLockError::Error(source)) => return Err(workspace_fs_error(&path, source)),
        }

        // The path is persistent and intentionally empty. The OS lock on the
        // file handle is the only ownership primitive.
        file.set_len(0)
            .map_err(|source| workspace_fs_error(&path, source))?;
        Ok(Box::new(FileSystemWorkspaceLockLease {
            path,
            file: Some(file),
        }))
    }
}

#[derive(Debug)]
struct FileSystemWorkspaceLockLease {
    path: PathBuf,
    file: Option<fs::File>,
}

#[async_trait]
impl WorkspaceLockLease for FileSystemWorkspaceLockLease {
    fn release(&mut self) -> WorkspaceResult<()> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        if let Err(source) = file.unlock() {
            self.file = Some(file);
            return Err(workspace_fs_error(&self.path, source));
        }
        Ok(())
    }
}

impl Drop for FileSystemWorkspaceLockLease {
    fn drop(&mut self) {
        let _ = self.release();
    }
}
