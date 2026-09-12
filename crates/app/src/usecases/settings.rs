use crate::mapping::{workspace_settings_to_domain, workspace_settings_to_dto};
use crate::ports::SharedWorkspaceSettings;
use crate::{AppError, AppResult};
use atelier_adapter_database::DatabaseSettingsRepository;
use atelier_app_api::settings::{
    ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use atelier_settings::WorkspaceSettingsService;

pub struct SettingsUseCases<'a> {
    pub(crate) settings: &'a WorkspaceSettingsService<DatabaseSettingsRepository>,
    pub(crate) settings_state: &'a SharedWorkspaceSettings,
}

impl SettingsUseCases<'_> {
    pub async fn get(&self) -> AppResult<WorkspaceSettingsDto> {
        self.settings
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
        self.settings
            .update_workspace_settings(settings)
            .await
            .map(|settings| {
                self.settings_state.replace(settings.clone());
                workspace_settings_to_dto(&settings)
            })
            .map_err(AppError::from)
    }

    pub async fn reset(&self) -> AppResult<ResetWorkspaceSettingsResponseDto> {
        self.settings
            .reset_workspace_settings()
            .await
            .map(|settings| ResetWorkspaceSettingsResponseDto {
                settings: {
                    self.settings_state.replace(settings.clone());
                    workspace_settings_to_dto(&settings)
                },
            })
            .map_err(AppError::from)
    }
}
