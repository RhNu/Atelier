use atelier_app::CommandResult;
use atelier_app_api::agent::{
    AgentEventDto, AgentRegistryDto, AgentSessionDto, AgentWorkspaceSettingsDto,
    CreateAgentSessionRequestDto, DeleteAgentConnectionRequestDto, DeleteAgentModelRequestDto,
    DeleteAgentSessionRequestDto, DeleteAgentSessionResponseDto, ListAgentEventsRequestDto,
    RenameAgentSessionRequestDto, SaveAgentConnectionRequestDto, SaveAgentModelRequestDto,
    UpdateAgentWorkspaceSettingsRequestDto,
};
use tauri::State;

use crate::desktop::DesktopState;

#[tauri::command]
pub async fn get_agent_registry(state: State<'_, DesktopState>) -> CommandResult<AgentRegistryDto> {
    state.host.get_agent_registry().await
}

#[tauri::command]
pub async fn save_agent_connection(
    state: State<'_, DesktopState>,
    request: SaveAgentConnectionRequestDto,
) -> CommandResult<AgentRegistryDto> {
    state.host.save_agent_connection(request).await
}

#[tauri::command]
pub async fn delete_agent_connection(
    state: State<'_, DesktopState>,
    request: DeleteAgentConnectionRequestDto,
) -> CommandResult<AgentRegistryDto> {
    state.host.delete_agent_connection(request).await
}

#[tauri::command]
pub async fn save_agent_model(
    state: State<'_, DesktopState>,
    request: SaveAgentModelRequestDto,
) -> CommandResult<AgentRegistryDto> {
    state.host.save_agent_model(request).await
}

#[tauri::command]
pub async fn delete_agent_model(
    state: State<'_, DesktopState>,
    request: DeleteAgentModelRequestDto,
) -> CommandResult<AgentRegistryDto> {
    state.host.delete_agent_model(request).await
}

#[tauri::command]
pub async fn get_agent_workspace_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<AgentWorkspaceSettingsDto> {
    state.host.get_agent_workspace_settings().await
}

#[tauri::command]
pub async fn update_agent_workspace_settings(
    state: State<'_, DesktopState>,
    request: UpdateAgentWorkspaceSettingsRequestDto,
) -> CommandResult<AgentWorkspaceSettingsDto> {
    state.host.update_agent_workspace_settings(request).await
}

#[tauri::command]
pub async fn create_agent_session(
    state: State<'_, DesktopState>,
    request: CreateAgentSessionRequestDto,
) -> CommandResult<AgentSessionDto> {
    state.host.create_agent_session(request).await
}

#[tauri::command]
pub async fn list_agent_sessions(
    state: State<'_, DesktopState>,
) -> CommandResult<Vec<AgentSessionDto>> {
    state.host.list_agent_sessions().await
}

#[tauri::command]
pub async fn rename_agent_session(
    state: State<'_, DesktopState>,
    request: RenameAgentSessionRequestDto,
) -> CommandResult<AgentSessionDto> {
    state.host.rename_agent_session(request).await
}

#[tauri::command]
pub async fn delete_agent_session(
    state: State<'_, DesktopState>,
    request: DeleteAgentSessionRequestDto,
) -> CommandResult<DeleteAgentSessionResponseDto> {
    state.host.delete_agent_session(request).await
}

#[tauri::command]
pub async fn list_agent_events(
    state: State<'_, DesktopState>,
    request: ListAgentEventsRequestDto,
) -> CommandResult<Vec<AgentEventDto>> {
    state.host.list_agent_events(request).await
}
