use nai_atelier_app_api::settings::{
    ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Returns workspace-local settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or settings cannot be read.
    pub async fn get_workspace_settings(&self) -> CommandResult<WorkspaceSettingsDto> {
        Self::command_result(self.current_app()?.settings().get().await)
    }

    /// Replaces workspace-local settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, settings are invalid, or persistence fails.
    pub async fn update_workspace_settings(
        &self,
        request: UpdateWorkspaceSettingsRequestDto,
    ) -> CommandResult<WorkspaceSettingsDto> {
        Self::command_result(self.current_app()?.settings().update(request).await)
    }

    /// Resets workspace-local settings to v1 defaults.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or persistence fails.
    pub async fn reset_workspace_settings(
        &self,
    ) -> CommandResult<ResetWorkspaceSettingsResponseDto> {
        Self::command_result(self.current_app()?.settings().reset().await)
    }
}
