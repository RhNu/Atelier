use crate::{SettingsRepository, SettingsResult, WorkspaceSettings};

#[derive(Clone, Debug)]
pub struct SettingsService<R> {
    repository: R,
}

impl<R> SettingsService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> SettingsService<R>
where
    R: SettingsRepository,
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
