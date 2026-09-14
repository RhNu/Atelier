use std::net::IpAddr;

use url::{Host, Url};

use crate::{AgentError, AgentResult};

macro_rules! string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

string_id!(AgentConnectionId);
string_id!(AgentModelId);
string_id!(AgentSessionId);
string_id!(AgentEventId);
string_id!(AgentActionId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentAuth {
    None,
    Bearer { secret_record_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentConnection {
    pub id: AgentConnectionId,
    pub display_name: String,
    pub base_url: String,
    pub auth: AgentAuth,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl AgentConnection {
    /// Validates fields that are safe to persist and use for an OpenAI-compatible service.
    ///
    /// # Errors
    /// Returns an error for empty identifiers/names, invalid HTTP URLs, or empty secret references.
    pub fn validate(&self) -> AgentResult<()> {
        ensure_not_blank("connection.id", self.id.as_str())?;
        ensure_not_blank("connection.display_name", &self.display_name)?;
        let url = Url::parse(&self.base_url).map_err(|error| {
            AgentError::validation(format!("invalid connection base URL: {error}"))
        })?;
        if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
            return Err(AgentError::validation(
                "connection base URL must be an absolute HTTP or HTTPS URL",
            ));
        }
        if let AgentAuth::Bearer { secret_record_id } = &self.auth {
            ensure_not_blank("connection.auth.secret_record_id", secret_record_id)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn warns_about_insecure_remote_http(&self) -> bool {
        Url::parse(&self.base_url).is_ok_and(|url| {
            url.scheme() == "http"
                && url.host().is_some_and(|host| match host {
                    Host::Domain(name) => !name.eq_ignore_ascii_case("localhost"),
                    Host::Ipv4(address) => !IpAddr::V4(address).is_loopback(),
                    Host::Ipv6(address) => !IpAddr::V6(address).is_loopback(),
                })
        })
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AgentProbeStatus {
    #[default]
    Unknown,
    Verified,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentModel {
    pub id: AgentModelId,
    pub connection_id: AgentConnectionId,
    pub wire_model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub supports_vision: bool,
    pub probe_status: AgentProbeStatus,
    pub updated_at_ms: u64,
}

impl AgentModel {
    /// Validates a model definition independently of a concrete provider.
    ///
    /// # Errors
    /// Returns an error when identifiers are empty or token and temperature limits are invalid.
    pub fn validate(&self) -> AgentResult<()> {
        ensure_not_blank("model.id", self.id.as_str())?;
        ensure_not_blank("model.connection_id", self.connection_id.as_str())?;
        ensure_not_blank("model.wire_model_id", &self.wire_model_id)?;
        ensure_not_blank("model.display_name", &self.display_name)?;
        if self.context_window < 1_024 {
            return Err(AgentError::validation(
                "model context window must be at least 1024 tokens",
            ));
        }
        if self.max_output_tokens == 0 || self.max_output_tokens >= self.context_window {
            return Err(AgentError::validation(
                "model max output tokens must be positive and smaller than its context window",
            ));
        }
        if !self.temperature.is_finite() || !(0.0..=2.0).contains(&self.temperature) {
            return Err(AgentError::validation(
                "model temperature must be finite and between 0 and 2",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentRegistry {
    pub connections: Vec<AgentConnection>,
    pub models: Vec<AgentModel>,
}

impl AgentRegistry {
    /// Validates records and connection references.
    ///
    /// # Errors
    /// Returns an error for invalid records, duplicate ids, or orphaned models.
    pub fn validate(&self) -> AgentResult<()> {
        let mut connection_ids = std::collections::BTreeSet::new();
        for connection in &self.connections {
            connection.validate()?;
            if !connection_ids.insert(connection.id.as_str()) {
                return Err(AgentError::validation("duplicate agent connection id"));
            }
        }
        let mut model_ids = std::collections::BTreeSet::new();
        for model in &self.models {
            model.validate()?;
            if !model_ids.insert(model.id.as_str()) {
                return Err(AgentError::validation("duplicate agent model id"));
            }
            if !connection_ids.contains(model.connection_id.as_str()) {
                return Err(AgentError::validation(format!(
                    "agent model `{}` references a missing connection",
                    model.id.as_str()
                )));
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum AgentPermissionMode {
    #[default]
    Standard,
    Ask,
    BypassAll,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentWorkspaceSettings {
    pub display_name: String,
    pub instructions: String,
    pub v5_prompt_guidance: String,
    pub tag_prompt_guidance: String,
    pub output_vision_enabled: bool,
    pub permission_mode: AgentPermissionMode,
    pub default_model_id: Option<AgentModelId>,
}

impl Default for AgentWorkspaceSettings {
    fn default() -> Self {
        Self {
            display_name: "Atelier Agent".to_owned(),
            instructions: String::new(),
            v5_prompt_guidance: String::new(),
            tag_prompt_guidance: String::new(),
            output_vision_enabled: false,
            permission_mode: AgentPermissionMode::Standard,
            default_model_id: None,
        }
    }
}

impl AgentWorkspaceSettings {
    /// Validates user-editable workspace Agent instructions.
    ///
    /// # Errors
    /// Returns an error for a blank display name or instruction fields over their limits.
    pub fn validate(&self) -> AgentResult<()> {
        ensure_not_blank("workspace_agent.display_name", &self.display_name)?;
        ensure_max_chars("workspace_agent.display_name", &self.display_name, 80)?;
        ensure_max_chars("workspace_agent.instructions", &self.instructions, 16_000)?;
        ensure_max_chars(
            "workspace_agent.v5_prompt_guidance",
            &self.v5_prompt_guidance,
            8_000,
        )?;
        ensure_max_chars(
            "workspace_agent.tag_prompt_guidance",
            &self.tag_prompt_guidance,
            8_000,
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentModelSnapshot {
    pub model_id: AgentModelId,
    pub connection_id: AgentConnectionId,
    pub wire_model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub supports_vision: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentPersonaSnapshot {
    pub display_name: String,
    pub instructions: String,
    pub v5_prompt_guidance: String,
    pub tag_prompt_guidance: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentSessionStatus {
    Idle,
    Running,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentSession {
    pub id: AgentSessionId,
    pub title: String,
    pub model: AgentModelSnapshot,
    pub persona: AgentPersonaSnapshot,
    pub status: AgentSessionStatus,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentEventKind {
    UserMessage {
        content: String,
    },
    AssistantMessage {
        content: String,
        interrupted: bool,
    },
    ToolCall {
        tool_name: String,
        arguments_json: String,
    },
    ToolResult {
        tool_name: String,
        result_json: String,
        failed: bool,
    },
    Approval {
        tool_name: String,
        approved: bool,
    },
    Warning {
        content: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentEvent {
    pub id: AgentEventId,
    pub session_id: AgentSessionId,
    pub sequence: u64,
    pub created_at_ms: u64,
    pub kind: AgentEventKind,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentActionState {
    Applied,
    Undone,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentAction {
    pub id: AgentActionId,
    pub session_id: AgentSessionId,
    pub tool_name: String,
    pub base_revision: u64,
    pub applied_revision: u64,
    pub before_json: String,
    pub after_json: String,
    pub state: AgentActionState,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSummary {
    pub session_id: AgentSessionId,
    pub through_sequence: u64,
    pub content: String,
    pub content_hash: String,
    pub created_at_ms: u64,
}

fn ensure_not_blank(field: &str, value: &str) -> AgentResult<()> {
    if value.trim().is_empty() {
        Err(AgentError::validation(format!("{field} must not be blank")))
    } else {
        Ok(())
    }
}

fn ensure_max_chars(field: &str, value: &str, max: usize) -> AgentResult<()> {
    if value.chars().count() <= max {
        Ok(())
    } else {
        Err(AgentError::validation(format!(
            "{field} must contain at most {max} characters"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(base_url: &str) -> AgentConnection {
        AgentConnection {
            id: AgentConnectionId::new("connection"),
            display_name: "Connection".to_owned(),
            base_url: base_url.to_owned(),
            auth: AgentAuth::None,
            created_at_ms: 1,
            updated_at_ms: 1,
        }
    }

    #[test]
    fn warns_only_for_remote_plain_http() {
        assert!(!connection("http://localhost:1234/v1").warns_about_insecure_remote_http());
        assert!(!connection("http://127.0.0.1:1234/v1").warns_about_insecure_remote_http());
        assert!(!connection("https://models.example/v1").warns_about_insecure_remote_http());
        assert!(connection("http://models.example/v1").warns_about_insecure_remote_http());
    }

    #[test]
    fn model_output_must_fit_context() {
        let model = AgentModel {
            id: AgentModelId::new("model"),
            connection_id: AgentConnectionId::new("connection"),
            wire_model_id: "wire".to_owned(),
            display_name: "Model".to_owned(),
            context_window: 4_096,
            max_output_tokens: 4_096,
            temperature: 0.3,
            supports_vision: false,
            probe_status: AgentProbeStatus::Unknown,
            updated_at_ms: 1,
        };

        assert_eq!(
            model.validate().expect_err("invalid").kind,
            crate::AgentErrorKind::Validation
        );
    }
}
