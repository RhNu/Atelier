use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthKindDto {
    None,
    Bearer,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SaveAgentConnectionRequestDto {
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    pub auth_kind: AgentAuthKindDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

impl std::fmt::Debug for SaveAgentConnectionRequestDto {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SaveAgentConnectionRequestDto")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("base_url", &self.base_url)
            .field("auth_kind", &self.auth_kind)
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteAgentConnectionRequestDto {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct AgentConnectionDto {
    pub id: String,
    pub display_name: String,
    pub base_url: String,
    pub auth_kind: AgentAuthKindDto,
    pub secret_present: bool,
    pub insecure_remote_http: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AgentProbeStatusDto {
    Unknown,
    Verified,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SaveAgentModelRequestDto {
    pub id: String,
    pub connection_id: String,
    pub wire_model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub temperature: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteAgentModelRequestDto {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct AgentModelDto {
    pub id: String,
    pub connection_id: String,
    pub wire_model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub probe_status: AgentProbeStatusDto,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct AgentRegistryDto {
    pub connections: Vec<AgentConnectionDto>,
    pub models: Vec<AgentModelDto>,
}
