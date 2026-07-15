use super::{
    AppError, AppResult, ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto,
    WorkspaceSession, WorkspaceSettingsDto, workspace_settings_to_domain,
    workspace_settings_to_dto,
};

pub struct SettingsUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}

impl<S, F, E> SettingsUseCases<'_, S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    pub async fn get(&self) -> AppResult<WorkspaceSettingsDto> {
        self.app
            .inner
            .settings
            .get_workspace_settings()
            .await
            .map(|settings| workspace_settings_to_dto(&settings))
            .map_err(AppError::from)
    }

    pub async fn update(
        &self,
        request: UpdateWorkspaceSettingsRequestDto,
    ) -> AppResult<WorkspaceSettingsDto> {
        let settings = workspace_settings_to_domain(&request.settings)?;
        self.app
            .inner
            .settings
            .update_workspace_settings(settings)
            .await
            .map(|settings| {
                self.app.inner.settings_state.replace(settings.clone());
                workspace_settings_to_dto(&settings)
            })
            .map_err(AppError::from)
    }

    pub async fn reset(&self) -> AppResult<ResetWorkspaceSettingsResponseDto> {
        self.app
            .inner
            .settings
            .reset_workspace_settings()
            .await
            .map(|settings| ResetWorkspaceSettingsResponseDto {
                settings: {
                    self.app.inner.settings_state.replace(settings.clone());
                    workspace_settings_to_dto(&settings)
                },
            })
            .map_err(AppError::from)
    }
}
