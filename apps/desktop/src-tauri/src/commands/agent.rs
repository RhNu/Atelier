use atelier_app::CommandResult;
use atelier_app_api::agent::{
    AgentEventDto, AgentRegistryDto, AgentSessionDto, AgentTurnEventDto, AgentTurnResultDto,
    AgentWorkspaceSettingsDto, CancelAgentTurnRequestDto, CreateAgentSessionRequestDto,
    DecideAgentApprovalRequestDto, DeleteAgentConnectionRequestDto, DeleteAgentModelRequestDto,
    DeleteAgentSessionRequestDto, DeleteAgentSessionResponseDto, DiscoverAgentModelsRequestDto,
    DiscoveredAgentModelDto, ListAgentEventsRequestDto, RenameAgentSessionRequestDto,
    RunAgentTurnRequestDto, SaveAgentConnectionRequestDto, SaveAgentModelRequestDto,
    UndoAgentActionRequestDto, UpdateAgentWorkspaceSettingsRequestDto,
};
use atelier_app_api::generation::VersionedGenerationDraftDto;
use tauri::{State, ipc::Channel};

use crate::desktop::DesktopState;

#[tauri::command]
pub async fn get_agent_registry(state: State<'_, DesktopState>) -> CommandResult<AgentRegistryDto> {
    state.host.get_agent_registry().await
}

#[tauri::command]
pub async fn discover_agent_models(
    state: State<'_, DesktopState>,
    request: DiscoverAgentModelsRequestDto,
) -> CommandResult<Vec<DiscoveredAgentModelDto>> {
    state.host.discover_agent_models(request).await
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

#[tauri::command]
pub async fn run_agent_turn(
    state: State<'_, DesktopState>,
    request: RunAgentTurnRequestDto,
    on_event: Channel<AgentTurnEventDto>,
) -> CommandResult<AgentTurnResultDto> {
    let desktop = state.inner().clone();
    state
        .host
        .run_agent_turn(
            request,
            std::sync::Arc::new(move |event| {
                if let AgentTurnEventDto::GenerationSubmitted { directive } = &event {
                    desktop.kick_generation_worker(directive.clone());
                }
                let _ = on_event.send(event);
            }),
        )
        .await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri command state extractors are passed by value"
)]
pub fn decide_agent_approval(
    state: State<'_, DesktopState>,
    request: DecideAgentApprovalRequestDto,
) -> CommandResult<()> {
    state.host.decide_agent_approval(request)
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri command state extractors are passed by value"
)]
pub fn cancel_agent_turn(
    state: State<'_, DesktopState>,
    request: CancelAgentTurnRequestDto,
) -> CommandResult<bool> {
    state.host.cancel_agent_turn(request)
}

#[tauri::command]
pub async fn undo_agent_action(
    state: State<'_, DesktopState>,
    request: UndoAgentActionRequestDto,
) -> CommandResult<VersionedGenerationDraftDto> {
    state.host.undo_agent_action(request).await
}
