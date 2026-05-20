use async_trait::async_trait;

use crate::{SettingsResult, WorkspaceSettings};

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_workspace_settings(&self) -> SettingsResult<WorkspaceSettings>;

    async fn save_workspace_settings(&self, settings: WorkspaceSettings) -> SettingsResult<()>;

    async fn reset_workspace_settings(&self) -> SettingsResult<()>;
}
