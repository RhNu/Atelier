use async_trait::async_trait;

use crate::{
    WorkspaceLayout, WorkspaceLockMetadata, WorkspaceLockRequest, WorkspaceManifest,
    WorkspaceResult, WorkspaceRoot,
};

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
        request: WorkspaceLockRequest,
    ) -> WorkspaceResult<Box<dyn WorkspaceLockLease>>;
}

#[async_trait]
pub trait WorkspaceLockLease: Send {
    async fn metadata(&self) -> WorkspaceResult<WorkspaceLockMetadata>;
}
