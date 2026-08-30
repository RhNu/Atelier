use atelier_app_api::settings::{
    GlobalSettingsDto, ResetWorkspaceSettingsResponseDto, UpdateGlobalSettingsRequestDto,
    UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use atelier_image_analysis::ImageAnalysisModelId;

use crate::commands::{AtelierRuntime, CommandResult};
use crate::mapping::{
    global_frontend_settings_to_domain, global_safety_settings_to_domain, global_settings_to_dto,
};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Returns user-level application settings.
    ///
    /// # Errors
    /// Returns an error envelope when global settings cannot be read.
    pub async fn get_global_settings(&self) -> CommandResult<GlobalSettingsDto> {
        self.global_settings
            .get_global_settings()
            .await
            .map(|settings| global_settings_to_dto(&settings))
            .map_err(crate::AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Replaces editable user-level application preferences.
    ///
    /// # Errors
    /// Returns an error envelope when global settings cannot be read or written.
    pub async fn update_global_settings(
        &self,
        request: UpdateGlobalSettingsRequestDto,
    ) -> CommandResult<GlobalSettingsDto> {
        let wd_enabled = request.safety.wd_auto_review_enabled;
        if wd_enabled {
            let ready = self
                .downloadable_resources
                .as_ref()
                .is_some_and(|resources| resources.resolve("wd-swinv2-tagger-v3").is_ok());
            if !ready {
                return Err(crate::AppError::new(
                    "image_analysis_model_unavailable",
                    "WD automatic review requires an installed and verified WD model",
                )
                .envelope());
            }
        }

        let settings = self
            .global_settings
            .update_application_settings(
                global_frontend_settings_to_domain(request.frontend),
                global_safety_settings_to_domain(request.safety),
            )
            .await
            .map_err(crate::AppError::from)
            .map_err(|error| error.envelope())?;
        if let Some(control) = &self.safety_policy_control {
            control.set_wd_auto_review_enabled(wd_enabled);
        }
        if !wd_enabled && let Some(sessions) = &self.image_analysis_sessions {
            sessions
                .unload(ImageAnalysisModelId::WdSwinv2TaggerV3)
                .map_err(crate::AppError::from)
                .map_err(|error| error.envelope())?;
        }
        Ok(global_settings_to_dto(&settings))
    }

    /// Returns workspace-local settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or settings cannot be read.
    pub async fn get_workspace_settings(&self) -> CommandResult<WorkspaceSettingsDto> {
        Self::command_result(self.current_session()?.settings().get().await)
    }

    /// Replaces workspace-local settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, settings are invalid, or persistence fails.
    pub async fn update_workspace_settings(
        &self,
        request: UpdateWorkspaceSettingsRequestDto,
    ) -> CommandResult<WorkspaceSettingsDto> {
        Self::command_result(self.current_session()?.settings().update(request).await)
    }

    /// Resets workspace-local settings to v1 defaults.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or persistence fails.
    pub async fn reset_workspace_settings(
        &self,
    ) -> CommandResult<ResetWorkspaceSettingsResponseDto> {
        Self::command_result(self.current_session()?.settings().reset().await)
    }
}
