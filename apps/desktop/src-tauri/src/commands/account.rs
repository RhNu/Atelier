use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
    ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, SubscriptionSummaryDto,
    UpdateApiKeyRequestDto,
};
use tauri::State;

#[tauri::command]
pub async fn create_api_key(
    state: State<'_, DesktopState>,
    request: CreateApiKeyRequestDto,
) -> CommandResult<ApiKeyRecordDto> {
    state.host.create_api_key(request).await
}

#[tauri::command]
pub async fn update_api_key(
    state: State<'_, DesktopState>,
    request: UpdateApiKeyRequestDto,
) -> CommandResult<ApiKeyRecordDto> {
    state.host.update_api_key(request).await
}

#[tauri::command]
pub async fn delete_api_key(
    state: State<'_, DesktopState>,
    request: DeleteApiKeyRequestDto,
) -> CommandResult<DeleteApiKeyResponseDto> {
    state.host.delete_api_key(request).await
}

#[tauri::command]
pub async fn list_api_keys(state: State<'_, DesktopState>) -> CommandResult<Vec<ApiKeyRecordDto>> {
    state.host.list_api_keys().await
}

#[tauri::command]
pub async fn set_active_api_key(
    state: State<'_, DesktopState>,
    request: SetActiveApiKeyRequestDto,
) -> CommandResult<()> {
    state.host.set_active_api_key(request).await
}

#[tauri::command]
pub async fn probe_api_key(
    state: State<'_, DesktopState>,
    request: ProbeApiKeyRequestDto,
) -> CommandResult<SubscriptionSummaryDto> {
    state.host.probe_api_key(request).await
}

#[tauri::command]
pub async fn probe_active_api_key(
    state: State<'_, DesktopState>,
) -> CommandResult<SubscriptionSummaryDto> {
    state.host.probe_active_api_key().await
}
