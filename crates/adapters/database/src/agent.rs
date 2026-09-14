use async_trait::async_trait;
use atelier_agent::{
    AgentAction, AgentActionId, AgentActionState, AgentConnectionId, AgentError, AgentEvent,
    AgentEventId, AgentEventKind, AgentModelId, AgentModelSnapshot, AgentPermissionMode,
    AgentPersonaSnapshot, AgentResult, AgentSession, AgentSessionId, AgentSessionStatus,
    AgentSummary, AgentWorkspaceRepository, AgentWorkspaceSettings,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::connection::DatabaseConnection;

const SETTINGS_KEY: &str = "agent.settings";
const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct DatabaseAgentWorkspaceRepository {
    connection: DatabaseConnection,
}

impl DatabaseAgentWorkspaceRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl AgentWorkspaceRepository for DatabaseAgentWorkspaceRepository {
    async fn get_settings(&self) -> AgentResult<AgentWorkspaceSettings> {
        let connection = self.connection.lock().map_err(database_error)?;
        let json = connection
            .query_row(
                "SELECT value_json FROM workspace_settings WHERE setting_key = ?1",
                [SETTINGS_KEY],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?;
        drop(connection);
        json.map_or_else(
            || Ok(AgentWorkspaceSettings::default()),
            |json| decode::<StoredSettings>(&json)?.into_domain(),
        )
    }

    async fn save_settings(&self, settings: AgentWorkspaceSettings) -> AgentResult<()> {
        settings.validate()?;
        let json = encode(&StoredSettings::from_domain(&settings))?;
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                r"
                INSERT INTO workspace_settings(setting_key, value_json) VALUES (?1, ?2)
                ON CONFLICT(setting_key) DO UPDATE SET value_json = excluded.value_json
                ",
                params![SETTINGS_KEY, json],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn list_sessions(&self) -> AgentResult<Vec<AgentSession>> {
        let connection = self.connection.lock().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT session_id, title, status, model_json, persona_json,
                       created_at_ms, updated_at_ms
                FROM agent_sessions
                ORDER BY updated_at_ms DESC, session_id ASC
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], decode_session_row)
            .map_err(sql_error)?;
        let result = rows.collect::<Result<Vec<_>, _>>();
        drop(statement);
        drop(connection);
        result.map_err(sql_error)
    }

    async fn get_session(&self, id: &AgentSessionId) -> AgentResult<Option<AgentSession>> {
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .query_row(
                r"
                SELECT session_id, title, status, model_json, persona_json,
                       created_at_ms, updated_at_ms
                FROM agent_sessions WHERE session_id = ?1
                ",
                [id.as_str()],
                decode_session_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn save_session(&self, session: AgentSession) -> AgentResult<()> {
        let model_json = encode(&StoredModelSnapshot::from_domain(&session.model))?;
        let persona_json = encode(&StoredPersonaSnapshot::from_domain(&session.persona))?;
        let created_at_ms = write_u64(session.created_at_ms)?;
        let updated_at_ms = write_u64(session.updated_at_ms)?;
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                r"
                INSERT INTO agent_sessions(
                    session_id, title, status, model_json, persona_json, created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(session_id) DO UPDATE SET
                    title = excluded.title,
                    status = excluded.status,
                    model_json = excluded.model_json,
                    persona_json = excluded.persona_json,
                    updated_at_ms = excluded.updated_at_ms
                ",
                params![
                    session.id.as_str(),
                    session.title,
                    session_status_as_str(session.status),
                    model_json,
                    persona_json,
                    created_at_ms,
                    updated_at_ms,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn delete_session(&self, id: &AgentSessionId) -> AgentResult<bool> {
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                "DELETE FROM agent_sessions WHERE session_id = ?1",
                [id.as_str()],
            )
            .map(|count| count > 0)
            .map_err(sql_error)
    }

    async fn list_events(&self, session_id: &AgentSessionId) -> AgentResult<Vec<AgentEvent>> {
        let connection = self.connection.lock().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT event_id, sequence, event_json, created_at_ms
                FROM agent_events WHERE session_id = ?1 ORDER BY sequence ASC
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map([session_id.as_str()], |row| {
                let event_json = row.get::<_, String>(2)?;
                let stored = decode_sql::<StoredEventKind>(&event_json)?;
                Ok(AgentEvent {
                    id: AgentEventId::new(row.get::<_, String>(0)?),
                    session_id: session_id.clone(),
                    sequence: read_u64(row, 1)?,
                    kind: stored.into_domain(),
                    created_at_ms: read_u64(row, 3)?,
                })
            })
            .map_err(sql_error)?;
        let result = rows.collect::<Result<Vec<_>, _>>();
        drop(statement);
        drop(connection);
        result.map_err(sql_error)
    }

    async fn append_event(&self, event: AgentEvent) -> AgentResult<()> {
        let json = encode(&StoredEventKind::from_domain(&event.kind))?;
        let sequence = write_u64(event.sequence)?;
        let created_at_ms = write_u64(event.created_at_ms)?;
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                r"
                INSERT INTO agent_events(
                    event_id, session_id, sequence, event_kind, event_json, created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ",
                params![
                    event.id.as_str(),
                    event.session_id.as_str(),
                    sequence,
                    event_kind_as_str(&event.kind),
                    json,
                    created_at_ms,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn get_action(&self, id: &AgentActionId) -> AgentResult<Option<AgentAction>> {
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .query_row(
                r"
                SELECT session_id, tool_name, base_revision, applied_revision, before_json,
                       after_json, action_state, created_at_ms, updated_at_ms
                FROM agent_actions WHERE action_id = ?1
                ",
                [id.as_str()],
                |row| {
                    Ok(AgentAction {
                        id: id.clone(),
                        session_id: AgentSessionId::new(row.get::<_, String>(0)?),
                        tool_name: row.get(1)?,
                        base_revision: read_u64(row, 2)?,
                        applied_revision: read_u64(row, 3)?,
                        before_json: row.get(4)?,
                        after_json: row.get(5)?,
                        state: action_state_from_str(&row.get::<_, String>(6)?)?,
                        created_at_ms: read_u64(row, 7)?,
                        updated_at_ms: read_u64(row, 8)?,
                    })
                },
            )
            .optional()
            .map_err(sql_error)
    }

    async fn save_action(&self, action: AgentAction) -> AgentResult<()> {
        let base_revision = write_u64(action.base_revision)?;
        let applied_revision = write_u64(action.applied_revision)?;
        let created_at_ms = write_u64(action.created_at_ms)?;
        let updated_at_ms = write_u64(action.updated_at_ms)?;
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                r"
                INSERT INTO agent_actions(
                    action_id, session_id, tool_name, base_revision, applied_revision,
                    before_json, after_json, action_state, created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(action_id) DO UPDATE SET
                    action_state = excluded.action_state,
                    updated_at_ms = excluded.updated_at_ms
                ",
                params![
                    action.id.as_str(),
                    action.session_id.as_str(),
                    action.tool_name,
                    base_revision,
                    applied_revision,
                    action.before_json,
                    action.after_json,
                    action_state_as_str(action.state),
                    created_at_ms,
                    updated_at_ms,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn get_summary(&self, session_id: &AgentSessionId) -> AgentResult<Option<AgentSummary>> {
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .query_row(
                r"
                SELECT through_sequence, content, content_hash, created_at_ms
                FROM agent_summaries WHERE session_id = ?1
                ",
                [session_id.as_str()],
                |row| {
                    Ok(AgentSummary {
                        session_id: session_id.clone(),
                        through_sequence: read_u64(row, 0)?,
                        content: row.get(1)?,
                        content_hash: row.get(2)?,
                        created_at_ms: read_u64(row, 3)?,
                    })
                },
            )
            .optional()
            .map_err(sql_error)
    }

    async fn save_summary(&self, summary: AgentSummary) -> AgentResult<()> {
        let through_sequence = write_u64(summary.through_sequence)?;
        let created_at_ms = write_u64(summary.created_at_ms)?;
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                r"
                INSERT INTO agent_summaries(
                    session_id, through_sequence, content, content_hash, created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(session_id) DO UPDATE SET
                    through_sequence = excluded.through_sequence,
                    content = excluded.content,
                    content_hash = excluded.content_hash,
                    created_at_ms = excluded.created_at_ms
                ",
                params![
                    summary.session_id.as_str(),
                    through_sequence,
                    summary.content,
                    summary.content_hash,
                    created_at_ms,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn interrupt_running_sessions(&self, updated_at_ms: u64) -> AgentResult<usize> {
        let updated_at_ms = write_u64(updated_at_ms)?;
        let connection = self.connection.lock().map_err(database_error)?;
        connection
            .execute(
                "UPDATE agent_sessions SET status = 'interrupted', updated_at_ms = ?1 WHERE status = 'running'",
                [updated_at_ms],
            )
            .map_err(sql_error)
    }
}

fn decode_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentSession> {
    let model = decode_sql::<StoredModelSnapshot>(&row.get::<_, String>(3)?)?.into_domain();
    let persona = decode_sql::<StoredPersonaSnapshot>(&row.get::<_, String>(4)?)?.into_domain();
    Ok(AgentSession {
        id: AgentSessionId::new(row.get::<_, String>(0)?),
        title: row.get(1)?,
        status: session_status_from_str(&row.get::<_, String>(2)?)?,
        model,
        persona,
        created_at_ms: read_u64(row, 5)?,
        updated_at_ms: read_u64(row, 6)?,
    })
}

#[derive(Deserialize, Serialize)]
struct StoredSettings {
    schema_version: u32,
    display_name: String,
    instructions: String,
    v5_prompt_guidance: String,
    tag_prompt_guidance: String,
    permission_mode: String,
    default_model_id: Option<String>,
}

impl StoredSettings {
    fn from_domain(value: &AgentWorkspaceSettings) -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            display_name: value.display_name.clone(),
            instructions: value.instructions.clone(),
            v5_prompt_guidance: value.v5_prompt_guidance.clone(),
            tag_prompt_guidance: value.tag_prompt_guidance.clone(),
            permission_mode: permission_mode_as_str(value.permission_mode).to_owned(),
            default_model_id: value
                .default_model_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
        }
    }

    fn into_domain(self) -> AgentResult<AgentWorkspaceSettings> {
        if self.schema_version != SETTINGS_SCHEMA_VERSION {
            return Err(AgentError::repository(format!(
                "unsupported workspace Agent settings schema version {}",
                self.schema_version
            )));
        }
        let value = AgentWorkspaceSettings {
            display_name: self.display_name,
            instructions: self.instructions,
            v5_prompt_guidance: self.v5_prompt_guidance,
            tag_prompt_guidance: self.tag_prompt_guidance,
            permission_mode: permission_mode_from_str(&self.permission_mode)?,
            default_model_id: self.default_model_id.map(AgentModelId::new),
        };
        value.validate()?;
        Ok(value)
    }
}

#[derive(Deserialize, Serialize)]
struct StoredModelSnapshot {
    model_id: String,
    connection_id: String,
    wire_model_id: String,
    display_name: String,
    context_window: u32,
    max_output_tokens: u32,
    temperature: f32,
}

impl StoredModelSnapshot {
    fn from_domain(value: &AgentModelSnapshot) -> Self {
        Self {
            model_id: value.model_id.as_str().to_owned(),
            connection_id: value.connection_id.as_str().to_owned(),
            wire_model_id: value.wire_model_id.clone(),
            display_name: value.display_name.clone(),
            context_window: value.context_window,
            max_output_tokens: value.max_output_tokens,
            temperature: value.temperature,
        }
    }

    fn into_domain(self) -> AgentModelSnapshot {
        AgentModelSnapshot {
            model_id: AgentModelId::new(self.model_id),
            connection_id: AgentConnectionId::new(self.connection_id),
            wire_model_id: self.wire_model_id,
            display_name: self.display_name,
            context_window: self.context_window,
            max_output_tokens: self.max_output_tokens,
            temperature: self.temperature,
        }
    }
}

#[derive(Deserialize, Serialize)]
struct StoredPersonaSnapshot {
    display_name: String,
    instructions: String,
    v5_prompt_guidance: String,
    tag_prompt_guidance: String,
}

impl StoredPersonaSnapshot {
    fn from_domain(value: &AgentPersonaSnapshot) -> Self {
        Self {
            display_name: value.display_name.clone(),
            instructions: value.instructions.clone(),
            v5_prompt_guidance: value.v5_prompt_guidance.clone(),
            tag_prompt_guidance: value.tag_prompt_guidance.clone(),
        }
    }

    fn into_domain(self) -> AgentPersonaSnapshot {
        AgentPersonaSnapshot {
            display_name: self.display_name,
            instructions: self.instructions,
            v5_prompt_guidance: self.v5_prompt_guidance,
            tag_prompt_guidance: self.tag_prompt_guidance,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StoredEventKind {
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

impl StoredEventKind {
    fn from_domain(value: &AgentEventKind) -> Self {
        match value {
            AgentEventKind::UserMessage { content } => Self::UserMessage {
                content: content.clone(),
            },
            AgentEventKind::AssistantMessage {
                content,
                interrupted,
            } => Self::AssistantMessage {
                content: content.clone(),
                interrupted: *interrupted,
            },
            AgentEventKind::ToolCall {
                tool_name,
                arguments_json,
            } => Self::ToolCall {
                tool_name: tool_name.clone(),
                arguments_json: arguments_json.clone(),
            },
            AgentEventKind::ToolResult {
                tool_name,
                result_json,
                failed,
            } => Self::ToolResult {
                tool_name: tool_name.clone(),
                result_json: result_json.clone(),
                failed: *failed,
            },
            AgentEventKind::Approval {
                tool_name,
                approved,
            } => Self::Approval {
                tool_name: tool_name.clone(),
                approved: *approved,
            },
            AgentEventKind::Warning { content } => Self::Warning {
                content: content.clone(),
            },
        }
    }

    fn into_domain(self) -> AgentEventKind {
        match self {
            Self::UserMessage { content } => AgentEventKind::UserMessage { content },
            Self::AssistantMessage {
                content,
                interrupted,
            } => AgentEventKind::AssistantMessage {
                content,
                interrupted,
            },
            Self::ToolCall {
                tool_name,
                arguments_json,
            } => AgentEventKind::ToolCall {
                tool_name,
                arguments_json,
            },
            Self::ToolResult {
                tool_name,
                result_json,
                failed,
            } => AgentEventKind::ToolResult {
                tool_name,
                result_json,
                failed,
            },
            Self::Approval {
                tool_name,
                approved,
            } => AgentEventKind::Approval {
                tool_name,
                approved,
            },
            Self::Warning { content } => AgentEventKind::Warning { content },
        }
    }
}

fn encode(value: &impl Serialize) -> AgentResult<String> {
    serde_json::to_string(value).map_err(|error| AgentError::repository(error.to_string()))
}

fn decode<T: for<'de> Deserialize<'de>>(value: &str) -> AgentResult<T> {
    serde_json::from_str(value).map_err(|error| AgentError::repository(error.to_string()))
}

fn decode_sql<T: for<'de> Deserialize<'de>>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

const fn session_status_as_str(value: AgentSessionStatus) -> &'static str {
    match value {
        AgentSessionStatus::Idle => "idle",
        AgentSessionStatus::Running => "running",
        AgentSessionStatus::Interrupted => "interrupted",
    }
}

fn session_status_from_str(value: &str) -> rusqlite::Result<AgentSessionStatus> {
    match value {
        "idle" => Ok(AgentSessionStatus::Idle),
        "running" => Ok(AgentSessionStatus::Running),
        "interrupted" => Ok(AgentSessionStatus::Interrupted),
        _ => Err(invalid_sql_value("session status", value)),
    }
}

const fn action_state_as_str(value: AgentActionState) -> &'static str {
    match value {
        AgentActionState::Applied => "applied",
        AgentActionState::Undone => "undone",
    }
}

fn action_state_from_str(value: &str) -> rusqlite::Result<AgentActionState> {
    match value {
        "applied" => Ok(AgentActionState::Applied),
        "undone" => Ok(AgentActionState::Undone),
        _ => Err(invalid_sql_value("action state", value)),
    }
}

const fn permission_mode_as_str(value: AgentPermissionMode) -> &'static str {
    match value {
        AgentPermissionMode::Standard => "standard",
        AgentPermissionMode::Ask => "ask",
        AgentPermissionMode::BypassAll => "bypass_all",
    }
}

fn permission_mode_from_str(value: &str) -> AgentResult<AgentPermissionMode> {
    match value {
        "standard" => Ok(AgentPermissionMode::Standard),
        "ask" => Ok(AgentPermissionMode::Ask),
        "bypass_all" => Ok(AgentPermissionMode::BypassAll),
        _ => Err(AgentError::repository(format!(
            "unknown Agent permission mode `{value}`"
        ))),
    }
}

const fn event_kind_as_str(value: &AgentEventKind) -> &'static str {
    match value {
        AgentEventKind::UserMessage { .. } => "user_message",
        AgentEventKind::AssistantMessage { .. } => "assistant_message",
        AgentEventKind::ToolCall { .. } => "tool_call",
        AgentEventKind::ToolResult { .. } => "tool_result",
        AgentEventKind::Approval { .. } => "approval",
        AgentEventKind::Warning { .. } => "warning",
    }
}

fn invalid_sql_value(kind: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("unknown Agent {kind} `{value}`").into(),
    )
}

fn write_u64(value: u64) -> AgentResult<i64> {
    i64::try_from(value).map_err(|_| AgentError::repository("integer exceeds SQLite range"))
}

fn read_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn database_error(error: crate::DatabaseError) -> AgentError {
    let message = error.to_string();
    drop(error);
    AgentError::repository(message)
}
fn sql_error(error: rusqlite::Error) -> AgentError {
    let message = error.to_string();
    drop(error);
    AgentError::repository(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> AgentSession {
        AgentSession {
            id: AgentSessionId::new("session"),
            title: "First session".to_owned(),
            model: AgentModelSnapshot {
                model_id: AgentModelId::new("model"),
                connection_id: AgentConnectionId::new("connection"),
                wire_model_id: "wire-model".to_owned(),
                display_name: "Model".to_owned(),
                context_window: 32_768,
                max_output_tokens: 4_096,
                temperature: 0.3,
            },
            persona: AgentPersonaSnapshot {
                display_name: "Atelier Agent".to_owned(),
                instructions: "Be concise".to_owned(),
                v5_prompt_guidance: String::new(),
                tag_prompt_guidance: String::new(),
            },
            status: AgentSessionStatus::Running,
            created_at_ms: 10,
            updated_at_ms: 11,
        }
    }

    #[test]
    fn round_trips_workspace_agent_state_and_interrupts_running_sessions() {
        let connection = DatabaseConnection::open_memory().expect("database");
        let repository = DatabaseAgentWorkspaceRepository::new(connection);
        let settings = AgentWorkspaceSettings {
            permission_mode: AgentPermissionMode::Ask,
            instructions: "Keep characters separate".to_owned(),
            ..AgentWorkspaceSettings::default()
        };

        futures_executor::block_on(repository.save_settings(settings.clone())).expect("settings");
        assert_eq!(
            futures_executor::block_on(repository.get_settings()).expect("load settings"),
            settings
        );

        let session = session();
        futures_executor::block_on(repository.save_session(session.clone())).expect("session");
        let event = AgentEvent {
            id: AgentEventId::new("event"),
            session_id: session.id.clone(),
            sequence: 1,
            created_at_ms: 12,
            kind: AgentEventKind::UserMessage {
                content: "make it wider".to_owned(),
            },
        };
        futures_executor::block_on(repository.append_event(event.clone())).expect("event");
        assert_eq!(
            futures_executor::block_on(repository.list_events(&session.id)).expect("events"),
            vec![event]
        );

        assert_eq!(
            futures_executor::block_on(repository.interrupt_running_sessions(13))
                .expect("interrupt"),
            1
        );
        let loaded = futures_executor::block_on(repository.get_session(&session.id))
            .expect("load session")
            .expect("session exists");
        assert_eq!(loaded.status, AgentSessionStatus::Interrupted);
        assert_eq!(loaded.updated_at_ms, 13);
    }

    #[test]
    fn deleting_session_cascades_owned_actions_and_summary() {
        let connection = DatabaseConnection::open_memory().expect("database");
        let repository = DatabaseAgentWorkspaceRepository::new(connection);
        let session = session();
        futures_executor::block_on(repository.save_session(session.clone())).expect("session");
        let action = AgentAction {
            id: AgentActionId::new("action"),
            session_id: session.id.clone(),
            tool_name: "set_generation_size".to_owned(),
            base_revision: 1,
            applied_revision: 2,
            before_json: r#"{"width":832}"#.to_owned(),
            after_json: r#"{"width":1216}"#.to_owned(),
            state: AgentActionState::Applied,
            created_at_ms: 12,
            updated_at_ms: 12,
        };
        futures_executor::block_on(repository.save_action(action.clone())).expect("action");
        futures_executor::block_on(repository.save_summary(AgentSummary {
            session_id: session.id.clone(),
            through_sequence: 4,
            content: "summary".to_owned(),
            content_hash: "hash".to_owned(),
            created_at_ms: 13,
        }))
        .expect("summary");

        assert!(
            futures_executor::block_on(repository.delete_session(&session.id)).expect("delete")
        );
        assert!(
            futures_executor::block_on(repository.get_action(&action.id))
                .expect("action query")
                .is_none()
        );
        assert!(
            futures_executor::block_on(repository.get_summary(&session.id))
                .expect("summary query")
                .is_none()
        );
    }
}
