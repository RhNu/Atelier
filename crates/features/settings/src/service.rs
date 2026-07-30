use std::{path::PathBuf, sync::Arc};

use crate::{
    GlobalFrontendSettings, GlobalSafetySettings, GlobalSettings, GlobalSettingsRepository,
    SettingsResult, WorkspaceSettings, WorkspaceSettingsRepository,
};

#[derive(Clone, Debug)]
pub struct WorkspaceSettingsService<R> {
    repository: R,
}

impl<R> WorkspaceSettingsService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> WorkspaceSettingsService<R>
where
    R: WorkspaceSettingsRepository,
{
    /// Returns stored workspace settings or v1 defaults when no settings exist.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read.
    pub async fn get_workspace_settings(&self) -> SettingsResult<WorkspaceSettings> {
        self.repository.get_workspace_settings().await
    }

    /// Validates and replaces workspace settings.
    ///
    /// # Errors
    /// Returns an error when settings are invalid or persistence fails.
    pub async fn update_workspace_settings(
        &self,
        settings: WorkspaceSettings,
    ) -> SettingsResult<WorkspaceSettings> {
        settings.validate()?;
        self.repository
            .save_workspace_settings(settings.clone())
            .await?;
        Ok(settings)
    }

    /// Clears stored workspace settings and returns v1 defaults.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn reset_workspace_settings(&self) -> SettingsResult<WorkspaceSettings> {
        self.repository.reset_workspace_settings().await?;
        Ok(WorkspaceSettings::default())
    }
}

#[derive(Clone)]
pub struct GlobalSettingsService {
    repository: Arc<dyn GlobalSettingsRepository>,
}

impl GlobalSettingsService {
    #[must_use]
    pub fn new(repository: Arc<dyn GlobalSettingsRepository>) -> Self {
        Self { repository }
    }
    /// Returns user-level application settings.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read.
    pub async fn get_global_settings(&self) -> SettingsResult<GlobalSettings> {
        self.repository.get_global_settings().await
    }

    /// Replaces editable global frontend preferences without changing lifecycle state.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read or written.
    pub async fn update_frontend_settings(
        &self,
        frontend: GlobalFrontendSettings,
    ) -> SettingsResult<GlobalSettings> {
        let mut settings = self.repository.get_global_settings().await?;
        settings.frontend = frontend;
        self.repository
            .save_global_settings(settings.clone())
            .await?;
        Ok(settings)
    }

    /// Replaces editable application-wide frontend and safety preferences.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read or written.
    pub async fn update_application_settings(
        &self,
        frontend: GlobalFrontendSettings,
        safety: GlobalSafetySettings,
    ) -> SettingsResult<GlobalSettings> {
        let mut settings = self.repository.get_global_settings().await?;
        settings.frontend = frontend;
        settings.safety = safety;
        self.repository
            .save_global_settings(settings.clone())
            .await?;
        Ok(settings)
    }

    /// Records the workspace that should be restored on the next application start.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read or written.
    pub async fn record_last_workspace(
        &self,
        root: impl Into<PathBuf>,
    ) -> SettingsResult<GlobalSettings> {
        let mut settings = self.repository.get_global_settings().await?;
        settings.last_workspace = Some(root.into());
        self.repository
            .save_global_settings(settings.clone())
            .await?;
        Ok(settings)
    }
}
