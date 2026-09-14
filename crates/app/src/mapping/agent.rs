use atelier_agent::{
    AgentAuth, AgentConnection, AgentConnectionId, AgentEvent, AgentEventKind, AgentModel,
    AgentModelId, AgentPermissionMode, AgentProbeStatus, AgentRegistry, AgentSession,
    AgentSessionStatus, AgentWorkspaceSettings,
};
use atelier_app_api::agent::{
    AgentAuthKindDto, AgentConnectionDto, AgentEventDto, AgentEventKindDto, AgentModelDto,
    AgentModelSnapshotDto, AgentPermissionModeDto, AgentPersonaSnapshotDto, AgentProbeStatusDto,
    AgentRegistryDto, AgentSessionDto, AgentSessionStatusDto, AgentWorkspaceSettingsDto,
    SaveAgentConnectionRequestDto, SaveAgentModelRequestDto,
};

pub fn agent_connection_to_domain(
    request: SaveAgentConnectionRequestDto,
    current: Option<&AgentConnection>,
    now_ms: u64,
) -> AgentConnection {
    let secret_record_id = format!("agent-connection:{}", request.id);
    AgentConnection {
        id: AgentConnectionId::new(request.id),
        display_name: request.display_name,
        base_url: request.base_url,
        auth: match request.auth_kind {
            AgentAuthKindDto::None => AgentAuth::None,
            AgentAuthKindDto::Bearer => AgentAuth::Bearer { secret_record_id },
        },
        created_at_ms: current.map_or(now_ms, |value| value.created_at_ms),
        updated_at_ms: now_ms,
    }
}

pub fn agent_workspace_settings_to_domain(
    value: AgentWorkspaceSettingsDto,
) -> AgentWorkspaceSettings {
    AgentWorkspaceSettings {
        display_name: value.display_name,
        instructions: value.instructions,
        v5_prompt_guidance: value.v5_prompt_guidance,
        tag_prompt_guidance: value.tag_prompt_guidance,
        output_vision_enabled: value.output_vision_enabled,
        permission_mode: match value.permission_mode {
            AgentPermissionModeDto::Standard => AgentPermissionMode::Standard,
            AgentPermissionModeDto::Ask => AgentPermissionMode::Ask,
            AgentPermissionModeDto::BypassAll => AgentPermissionMode::BypassAll,
        },
        default_model_id: value.default_model_id.map(AgentModelId::new),
    }
}

pub fn agent_workspace_settings_to_dto(
    value: &AgentWorkspaceSettings,
) -> AgentWorkspaceSettingsDto {
    AgentWorkspaceSettingsDto {
        display_name: value.display_name.clone(),
        instructions: value.instructions.clone(),
        v5_prompt_guidance: value.v5_prompt_guidance.clone(),
        tag_prompt_guidance: value.tag_prompt_guidance.clone(),
        output_vision_enabled: value.output_vision_enabled,
        permission_mode: match value.permission_mode {
            AgentPermissionMode::Standard => AgentPermissionModeDto::Standard,
            AgentPermissionMode::Ask => AgentPermissionModeDto::Ask,
            AgentPermissionMode::BypassAll => AgentPermissionModeDto::BypassAll,
        },
        default_model_id: value
            .default_model_id
            .as_ref()
            .map(|id| id.as_str().to_owned()),
    }
}

pub fn agent_session_to_dto(value: &AgentSession) -> AgentSessionDto {
    AgentSessionDto {
        id: value.id.as_str().to_owned(),
        title: value.title.clone(),
        model: AgentModelSnapshotDto {
            model_id: value.model.model_id.as_str().to_owned(),
            connection_id: value.model.connection_id.as_str().to_owned(),
            wire_model_id: value.model.wire_model_id.clone(),
            display_name: value.model.display_name.clone(),
            context_window: value.model.context_window,
            max_output_tokens: value.model.max_output_tokens,
            temperature: value.model.temperature,
            supports_vision: value.model.supports_vision,
        },
        persona: AgentPersonaSnapshotDto {
            display_name: value.persona.display_name.clone(),
            instructions: value.persona.instructions.clone(),
            v5_prompt_guidance: value.persona.v5_prompt_guidance.clone(),
            tag_prompt_guidance: value.persona.tag_prompt_guidance.clone(),
        },
        status: match value.status {
            AgentSessionStatus::Idle => AgentSessionStatusDto::Idle,
            AgentSessionStatus::Running => AgentSessionStatusDto::Running,
            AgentSessionStatus::Interrupted => AgentSessionStatusDto::Interrupted,
        },
        created_at_ms: value.created_at_ms,
        updated_at_ms: value.updated_at_ms,
    }
}

pub fn agent_event_to_dto(value: &AgentEvent) -> AgentEventDto {
    AgentEventDto {
        id: value.id.as_str().to_owned(),
        session_id: value.session_id.as_str().to_owned(),
        sequence: value.sequence,
        created_at_ms: value.created_at_ms,
        event: match &value.kind {
            AgentEventKind::UserMessage { content } => AgentEventKindDto::UserMessage {
                content: content.clone(),
            },
            AgentEventKind::AssistantMessage {
                content,
                interrupted,
            } => AgentEventKindDto::AssistantMessage {
                content: content.clone(),
                interrupted: *interrupted,
            },
            AgentEventKind::ToolCall {
                tool_name,
                arguments_json,
            } => AgentEventKindDto::ToolCall {
                tool_name: tool_name.clone(),
                arguments_json: arguments_json.clone(),
            },
            AgentEventKind::ToolResult {
                tool_name,
                result_json,
                failed,
            } => AgentEventKindDto::ToolResult {
                tool_name: tool_name.clone(),
                result_json: result_json.clone(),
                failed: *failed,
            },
            AgentEventKind::Approval {
                tool_name,
                approved,
            } => AgentEventKindDto::Approval {
                tool_name: tool_name.clone(),
                approved: *approved,
            },
            AgentEventKind::Warning { content } => AgentEventKindDto::Warning {
                content: content.clone(),
            },
        },
    }
}

pub fn agent_model_to_domain(request: SaveAgentModelRequestDto, now_ms: u64) -> AgentModel {
    AgentModel {
        id: AgentModelId::new(request.id),
        connection_id: AgentConnectionId::new(request.connection_id),
        wire_model_id: request.wire_model_id,
        display_name: request.display_name,
        context_window: request.context_window,
        max_output_tokens: request.max_output_tokens,
        temperature: request.temperature,
        supports_vision: request.supports_vision,
        probe_status: AgentProbeStatus::Unknown,
        updated_at_ms: now_ms,
    }
}

pub fn agent_registry_to_dto(registry: &AgentRegistry) -> AgentRegistryDto {
    AgentRegistryDto {
        connections: registry.connections.iter().map(connection_to_dto).collect(),
        models: registry.models.iter().map(model_to_dto).collect(),
    }
}

fn connection_to_dto(value: &AgentConnection) -> AgentConnectionDto {
    AgentConnectionDto {
        id: value.id.as_str().to_owned(),
        display_name: value.display_name.clone(),
        base_url: value.base_url.clone(),
        auth_kind: match value.auth {
            AgentAuth::None => AgentAuthKindDto::None,
            AgentAuth::Bearer { .. } => AgentAuthKindDto::Bearer,
        },
        secret_present: matches!(value.auth, AgentAuth::Bearer { .. }),
        insecure_remote_http: value.warns_about_insecure_remote_http(),
    }
}

fn model_to_dto(value: &AgentModel) -> AgentModelDto {
    AgentModelDto {
        id: value.id.as_str().to_owned(),
        connection_id: value.connection_id.as_str().to_owned(),
        wire_model_id: value.wire_model_id.clone(),
        display_name: value.display_name.clone(),
        context_window: value.context_window,
        max_output_tokens: value.max_output_tokens,
        temperature: value.temperature,
        supports_vision: value.supports_vision,
        probe_status: match value.probe_status {
            AgentProbeStatus::Unknown => AgentProbeStatusDto::Unknown,
            AgentProbeStatus::Verified => AgentProbeStatusDto::Verified,
            AgentProbeStatus::Failed => AgentProbeStatusDto::Failed,
        },
    }
}
