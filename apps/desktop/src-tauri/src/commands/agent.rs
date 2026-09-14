use atelier_app::CommandResult;
use atelier_app_api::agent::{
    AgentRegistryDto, DeleteAgentConnectionRequestDto, DeleteAgentModelRequestDto,
    SaveAgentConnectionRequestDto, SaveAgentModelRequestDto,
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
