use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::settings::{
    FrontendLanguageDto, GlobalSettingsDto, ResetWorkspaceSettingsResponseDto,
    UpdateGlobalSettingsRequestDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use tauri::State;

#[tauri::command]
pub async fn get_global_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<GlobalSettingsDto> {
    state.host.get_global_settings().await
}

#[tauri::command]
pub async fn update_global_settings(
    state: State<'_, DesktopState>,
    request: UpdateGlobalSettingsRequestDto,
) -> CommandResult<GlobalSettingsDto> {
    let settings = state.host.update_global_settings(request).await?;
    state.set_notification_language(settings.frontend.language);
    Ok(settings)
}

// Tauri injects managed `State` command arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn set_notification_language(state: State<'_, DesktopState>, language: FrontendLanguageDto) {
    state.set_notification_language(language);
}

#[tauri::command]
pub async fn get_workspace_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<WorkspaceSettingsDto> {
    state.host.get_workspace_settings().await
}

#[tauri::command]
pub async fn update_workspace_settings(
    state: State<'_, DesktopState>,
    request: UpdateWorkspaceSettingsRequestDto,
) -> CommandResult<WorkspaceSettingsDto> {
    state.host.update_workspace_settings(request).await
}

#[tauri::command]
pub async fn reset_workspace_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<ResetWorkspaceSettingsResponseDto> {
    state.host.reset_workspace_settings().await
}
