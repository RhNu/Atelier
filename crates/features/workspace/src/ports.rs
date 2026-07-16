use async_trait::async_trait;

use crate::{WorkspaceLayout, WorkspaceManifest, WorkspaceResult, WorkspaceRoot};

#[async_trait]
pub trait WorkspaceStore: Send + Sync {
    async fn initialize(
        &self,
        root: &WorkspaceRoot,
        layout: &WorkspaceLayout,
    ) -> WorkspaceResult<WorkspaceManifest>;

    async fn load_manifest(
        &self,
        root: &WorkspaceRoot,
        layout: &WorkspaceLayout,
    ) -> WorkspaceResult<WorkspaceManifest>;
}

#[async_trait]
pub trait WorkspaceLock: Send + Sync {
    async fn acquire(
        &self,
        root: &WorkspaceRoot,
        layout: &WorkspaceLayout,
    ) -> WorkspaceResult<Box<dyn WorkspaceLockLease>>;
}

#[async_trait]
pub trait WorkspaceLockLease: Send {
    /// Releases the operating-system lock while the lease is still owned.
    ///
    /// Implementations must make this operation idempotent so shutdown can
    /// safely retry or run after an earlier explicit close.
    ///
    /// # Errors
    /// Returns an error when the underlying operating-system lock cannot be released.
    fn release(&mut self) -> WorkspaceResult<()>;
}
