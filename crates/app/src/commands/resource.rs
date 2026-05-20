use nai_atelier_app_api::resource::{
    ImportImageResourceRequestDto, ImportImageResourceResponseDto,
};

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
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
        Self::command_result(self.current_app()?.resources().import_image(request).await)
    }
}
