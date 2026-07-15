use async_trait::async_trait;

use crate::{GlobalSettings, SettingsResult, WorkspaceSettings};

#[async_trait]
pub trait WorkspaceSettingsRepository: Send + Sync {
    async fn get_workspace_settings(&self) -> SettingsResult<WorkspaceSettings>;

    async fn save_workspace_settings(&self, settings: WorkspaceSettings) -> SettingsResult<()>;

    async fn reset_workspace_settings(&self) -> SettingsResult<()>;
}

#[async_trait]
pub trait GlobalSettingsRepository: Send + Sync {
    async fn get_global_settings(&self) -> SettingsResult<GlobalSettings>;

    async fn save_global_settings(&self, settings: GlobalSettings) -> SettingsResult<()>;
}
