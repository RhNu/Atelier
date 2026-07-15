use atelier_app_api::resource::{
    GetResourceImageRequestDto, ImportImageResourceRequestDto, ImportImageResourceResponseDto,
    ReleaseImportedImageResourcesRequestDto, ReleaseImportedImageResourcesResponseDto,
    ResourceImageDto,
};

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Imports a workspace-scoped image resource for generation or drawing workflows.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, base64 is invalid, or resource storage fails.
    pub async fn import_image_resource(
        &self,
        request: ImportImageResourceRequestDto,
    ) -> CommandResult<ImportImageResourceResponseDto> {
        Self::command_result(
            self.current_session()?
                .resources()
                .import_image(request)
                .await,
        )
    }

    /// Reads a catalog image resource or variant as base64 for UI display.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the resource cannot be read.
    pub async fn get_resource_image(
        &self,
        request: GetResourceImageRequestDto,
    ) -> CommandResult<ResourceImageDto> {
        Self::command_result(self.current_session()?.resources().get_image(request).await)
    }

    /// Releases temporary user-selected image resources and their backing blobs.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or resource cleanup fails.
    pub async fn release_imported_image_resources(
        &self,
        request: ReleaseImportedImageResourcesRequestDto,
    ) -> CommandResult<ReleaseImportedImageResourcesResponseDto> {
        Self::command_result(
            self.current_session()?
                .resources()
                .release_imported_images(request)
                .await,
        )
    }
}
