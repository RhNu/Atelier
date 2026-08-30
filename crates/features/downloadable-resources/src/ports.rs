use async_trait::async_trait;

use crate::{
    DownloadableResourceCatalog, DownloadableResourceResult, DownloadableResourceStatus,
    InstalledResource, ResourceInstallProgress,
};

pub trait ResourceInstallProgressSink: Send + Sync {
    fn report(&self, progress: ResourceInstallProgress);
}

#[async_trait]
pub trait DownloadableResourceManager: Send + Sync {
    /// Returns whether the first-run resource choice has been completed or skipped.
    ///
    /// # Errors
    /// Returns an error when persisted manager state cannot be read.
    fn onboarding_complete(&self) -> DownloadableResourceResult<bool>;

    /// Persists completion of the first-run resource choice.
    ///
    /// # Errors
    /// Returns an error when persisted manager state cannot be written.
    fn complete_onboarding(&self) -> DownloadableResourceResult<()>;

    async fn catalog(
        &self,
        refresh: bool,
    ) -> DownloadableResourceResult<DownloadableResourceCatalog>;

    async fn statuses(&self) -> DownloadableResourceResult<Vec<DownloadableResourceStatus>>;

    async fn install(
        &self,
        resource_id: &str,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> DownloadableResourceResult<DownloadableResourceStatus>;

    async fn install_group(
        &self,
        group_id: &str,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> DownloadableResourceResult<Vec<DownloadableResourceStatus>>;

    /// Requests cancellation of an active resource installation.
    ///
    /// # Errors
    /// Returns an error when installer state is unavailable.
    fn cancel_install(&self, resource_id: &str) -> DownloadableResourceResult<()>;

    async fn delete(&self, resource_id: &str) -> DownloadableResourceResult<()>;

    /// Resolves an active verified resource and retains its version lease.
    ///
    /// # Errors
    /// Returns an error when the resource is not installed or its state is unavailable.
    fn resolve(&self, resource_id: &str) -> DownloadableResourceResult<InstalledResource>;
}
