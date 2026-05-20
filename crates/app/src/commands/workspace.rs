use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::workspace::{
    CloseWorkspaceResponseDto, OpenWorkspaceRequestDto, WorkspaceStatusDto,
};
use nai_atelier_secrets::SecretStore;
use nai_atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    /// Opens a workspace and replaces any existing command session.
    ///
    /// # Errors
    /// Returns an error envelope when workspace initialization, locking, database setup, or lexicon loading fails.
    pub async fn open_workspace(
        &self,
        request: OpenWorkspaceRequestDto,
    ) -> CommandResult<WorkspaceStatusDto> {
        let app = self.open_app(request).await?;
        Ok(app.workspace().status())
    }
}

impl<S, F, E> AppCommandHost<S, F, E> {
    /// Returns the current workspace status.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open.
    pub fn workspace_status(&self) -> CommandResult<WorkspaceStatusDto> {
        Ok(self.current_app()?.workspace().status())
    }

    /// Closes the current workspace session, if any.
    ///
    /// # Errors
    /// Returns an error envelope when command session state is unavailable.
    pub fn close_workspace(&self) -> CommandResult<CloseWorkspaceResponseDto> {
        let was_open = self.lock_session()?.take().is_some();
        Ok(CloseWorkspaceResponseDto { was_open })
    }
}
