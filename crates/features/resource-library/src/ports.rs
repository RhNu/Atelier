use async_trait::async_trait;

use crate::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    ResourceLibraryResult,
};

#[async_trait]
pub trait ResourceLibraryRepository: Send + Sync {
    async fn list_folders(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<Vec<LibraryFolder>>;

    async fn get_resource(
        &self,
        id: &LibraryResourceId,
    ) -> ResourceLibraryResult<Option<LibraryResource>>;

    async fn list_resources(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<Vec<LibraryResource>>;

    async fn save_folder(&self, folder: LibraryFolder) -> ResourceLibraryResult<()>;

    async fn save_resource(&self, resource: LibraryResource) -> ResourceLibraryResult<()>;

    async fn delete_folder(&self, id: &LibraryFolderId) -> ResourceLibraryResult<()>;

    async fn delete_resource(&self, id: &LibraryResourceId) -> ResourceLibraryResult<()>;
}
