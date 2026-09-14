use atelier_agent::{
    AgentAuth, AgentConnection, AgentConnectionId, AgentModel, AgentModelId, AgentProbeStatus,
    AgentRegistry,
};
use atelier_app_api::agent::{
    AgentAuthKindDto, AgentConnectionDto, AgentModelDto, AgentProbeStatusDto, AgentRegistryDto,
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

pub fn agent_model_to_domain(request: SaveAgentModelRequestDto, now_ms: u64) -> AgentModel {
    AgentModel {
        id: AgentModelId::new(request.id),
        connection_id: AgentConnectionId::new(request.connection_id),
        wire_model_id: request.wire_model_id,
        display_name: request.display_name,
        context_window: request.context_window,
        max_output_tokens: request.max_output_tokens,
        temperature: request.temperature,
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
        probe_status: match value.probe_status {
            AgentProbeStatus::Unknown => AgentProbeStatusDto::Unknown,
            AgentProbeStatus::Verified => AgentProbeStatusDto::Verified,
            AgentProbeStatus::Failed => AgentProbeStatusDto::Failed,
        },
    }
}
