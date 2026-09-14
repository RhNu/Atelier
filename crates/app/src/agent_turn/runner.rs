use std::{
    fmt::Write as _,
    sync::{Arc, Mutex},
};

use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{
    AgentAuth, AgentChatMessage, AgentChatRole, AgentErrorKind, AgentEvent, AgentEventId,
    AgentEventKind, AgentPromptFamily, AgentResolvedConnection, AgentRunObserver, AgentRunRequest,
    AgentSessionId, AgentSessionStatus, AgentSummary, build_agent_system_prompt,
};
use atelier_app_api::agent::{
    AgentTurnEventDto, AgentTurnResultDto, RunAgentTurnRequestDto, UndoAgentActionRequestDto,
};
use atelier_app_api::generation::{SaveGenerationDraftRequestDto, VersionedGenerationDraftDto};
use atelier_secrets::{SecretRecordId, SecretStore};
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use sha2::{Digest, Sha256};

use super::tools::{AgentTools, runtime_event_to_dto, tool_specs};
use crate::{AppError, AppResult, AtelierRuntime, WorkspaceSession};

#[allow(
    clippy::too_many_lines,
    reason = "one Agent turn keeps preflight, persistence, streaming, and finalization ordering visible"
)]
pub async fn run_agent_turn<S, F, E>(
    runtime: &AtelierRuntime<S, F, E>,
    request: RunAgentTurnRequestDto,
    output: Arc<dyn Fn(AgentTurnEventDto) + Send + Sync>,
) -> AppResult<AgentTurnResultDto>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    if request.message.trim().is_empty() {
        return Err(AppError::new(
            "agent_validation",
            "agent message must not be blank",
        ));
    }
    let app = runtime
        .current_session()
        .map_err(|error| AppError::new(error.code, error.message))?;
    let session_id = AgentSessionId::new(request.session_id.clone());
    let mut session = app.agent.get_session(&session_id).await?;
    let existing_events = app.agent.list_events(&session_id).await?;
    let user_sequence = next_sequence(&existing_events);
    let summary = refresh_summary(&app, &session_id, &existing_events).await?;
    let history = chat_history(&existing_events, summary.as_ref());
    let draft = app.generation().get_draft().await?;
    let context = serde_json::to_string(&serde_json::json!({
        "route": request.context.route,
        "selected_resource_ids": request.context.selected_resource_ids,
        "generation_draft": draft,
    }))
    .map_err(|error| AppError::new("agent_runtime", error.to_string()))?;
    let image_model = draft.as_ref().map(|value| value.draft.model);
    let family = if image_model
        .map(crate::mapping::image_model_to_domain)
        .is_some_and(|model| model.capabilities().uses_v5_extensions)
    {
        AgentPromptFamily::V5
    } else {
        AgentPromptFamily::Tags
    };
    let system_prompt = build_agent_system_prompt(&session.persona, family, &context);
    let connection = resolve_connection(runtime, &session.model.connection_id).await?;
    let permission_mode = app.agent.get_settings().await?.permission_mode;
    let cancellation = app.agent_turn.begin(session_id.as_str())?;
    let guard = TurnGuard {
        coordinator: app.agent_turn.clone(),
        session_id: request.session_id.clone(),
    };
    session.status = AgentSessionStatus::Running;
    session.updated_at_ms = crate::time::unix_timestamp_ms();
    app.agent.save_session(session.clone()).await?;
    output(AgentTurnEventDto::Started {
        session_id: request.session_id.clone(),
    });
    if let Err(error) = append_event(
        &app,
        &session_id,
        user_sequence,
        AgentEventKind::UserMessage {
            content: request.message.clone(),
        },
    )
    .await
    {
        finish_session(&app, &mut session, true).await?;
        return Err(error);
    }
    let assistant_buffer = Arc::new(Mutex::new(String::new()));
    let runtime_observer: Arc<dyn AgentRunObserver> = {
        let output = output.clone();
        let assistant_buffer = assistant_buffer.clone();
        Arc::new(move |event| {
            if let atelier_agent::AgentRuntimeEvent::AssistantTextDelta { text } = &event
                && let Ok(mut buffer) = assistant_buffer.lock()
            {
                buffer.push_str(text);
            }
            output(runtime_event_to_dto(event));
        })
    };
    let tools = Arc::new(AgentTools::new(
        app.clone(),
        session_id.clone(),
        permission_mode,
        app.agent_turn.clone(),
        runtime_observer.clone(),
        cancellation.clone(),
        user_sequence.saturating_add(1),
    ));
    let outcome = runtime
        .agent_runtime
        .run(
            AgentRunRequest {
                connection,
                model: session.model.clone(),
                system_prompt,
                user_message: request.message,
                history,
                tools: tool_specs(),
            },
            tools,
            runtime_observer,
            cancellation,
        )
        .await;
    let (outcome, interrupted) = match outcome {
        Ok(outcome) => (outcome, false),
        Err(error) if error.kind == AgentErrorKind::Cancelled => (
            atelier_agent::AgentRunOutcome {
                assistant_message: assistant_buffer
                    .lock()
                    .map_or_else(|_| String::new(), |value| value.clone()),
                ..Default::default()
            },
            true,
        ),
        Err(error) => {
            finish_session(&app, &mut session, true).await?;
            output(AgentTurnEventDto::Completed { interrupted: true });
            drop(guard);
            return Err(AppError::from(error));
        }
    };
    let final_events = match app.agent.list_events(&session_id).await {
        Ok(events) => events,
        Err(error) => {
            finish_session(&app, &mut session, true).await?;
            return Err(error.into());
        }
    };
    let append_result = append_event(
        &app,
        &session_id,
        next_sequence(&final_events),
        AgentEventKind::AssistantMessage {
            content: outcome.assistant_message.clone(),
            interrupted,
        },
    )
    .await;
    if let Err(error) = append_result {
        finish_session(&app, &mut session, true).await?;
        return Err(error);
    }
    finish_session(&app, &mut session, interrupted).await?;
    output(AgentTurnEventDto::Completed { interrupted });
    drop(guard);
    Ok(AgentTurnResultDto {
        session_id: request.session_id,
        assistant_message: outcome.assistant_message,
        interrupted,
        input_tokens: outcome.input_tokens,
        output_tokens: outcome.output_tokens,
        model_calls: outcome.model_calls,
    })
}

pub async fn undo_agent_action<S, F, E>(
    runtime: &AtelierRuntime<S, F, E>,
    request: UndoAgentActionRequestDto,
) -> AppResult<VersionedGenerationDraftDto>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    let app = runtime
        .current_session()
        .map_err(|error| AppError::new(error.code, error.message))?;
    let mut action = app
        .agent
        .get_action(&atelier_agent::AgentActionId::new(request.action_id))
        .await?
        .ok_or_else(|| AppError::new("agent_not_found", "agent action does not exist"))?;
    if action.state != atelier_agent::AgentActionState::Applied {
        return Err(AppError::new(
            "agent_conflict",
            "agent action was already undone",
        ));
    }
    let current = app.generation().get_draft().await?.ok_or_else(|| {
        AppError::new(
            "generation_draft_not_found",
            "generation draft does not exist",
        )
    })?;
    if current.revision != action.applied_revision {
        return Err(AppError::new(
            "agent_conflict",
            "generation draft changed after this action and cannot be safely undone",
        ));
    }
    let draft = serde_json::from_str(&action.before_json)
        .map_err(|error| AppError::new("agent_repository", error.to_string()))?;
    let restored = app
        .generation()
        .save_draft(SaveGenerationDraftRequestDto {
            expected_revision: current.revision,
            draft,
        })
        .await?;
    action.state = atelier_agent::AgentActionState::Undone;
    action.updated_at_ms = crate::time::unix_timestamp_ms();
    app.agent.save_action(action).await?;
    Ok(restored)
}

async fn resolve_connection<S, F, E>(
    runtime: &AtelierRuntime<S, F, E>,
    id: &atelier_agent::AgentConnectionId,
) -> AppResult<AgentResolvedConnection>
where
    S: SecretStore + Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    let registry = runtime.agent_registry.get_registry().await?;
    let connection = registry
        .connections
        .iter()
        .find(|value| &value.id == id)
        .ok_or_else(|| AppError::new("agent_not_found", "agent connection does not exist"))?;
    let bearer_token = match &connection.auth {
        AgentAuth::None => None,
        AgentAuth::Bearer { secret_record_id } => Some(
            runtime
                .secrets
                .read_secret(&SecretRecordId::new(secret_record_id.clone()))
                .await?
                .expose_secret()
                .to_owned(),
        ),
    };
    Ok(AgentResolvedConnection {
        base_url: connection.base_url.clone(),
        bearer_token,
    })
}

async fn finish_session<S, F, E>(
    app: &WorkspaceSession<S, F, E>,
    session: &mut atelier_agent::AgentSession,
    interrupted: bool,
) -> AppResult<()>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    session.status = if interrupted {
        AgentSessionStatus::Interrupted
    } else {
        AgentSessionStatus::Idle
    };
    session.updated_at_ms = crate::time::unix_timestamp_ms();
    app.agent
        .save_session(session.clone())
        .await
        .map_err(Into::into)
}

async fn append_event<S, F, E>(
    app: &WorkspaceSession<S, F, E>,
    session_id: &AgentSessionId,
    sequence: u64,
    kind: AgentEventKind,
) -> AppResult<()>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    app.agent
        .append_event(AgentEvent {
            id: AgentEventId::new(uuid::Uuid::new_v4().to_string()),
            session_id: session_id.clone(),
            sequence,
            created_at_ms: crate::time::unix_timestamp_ms(),
            kind,
        })
        .await
        .map_err(Into::into)
}

fn next_sequence(events: &[AgentEvent]) -> u64 {
    events
        .last()
        .map_or(0, |event| event.sequence.saturating_add(1))
}

fn chat_history(events: &[AgentEvent], summary: Option<&AgentSummary>) -> Vec<AgentChatMessage> {
    let mut history = Vec::new();
    if let Some(summary) = summary {
        history.push(AgentChatMessage {
            role: AgentChatRole::User,
            content: format!("Earlier conversation summary:\n{}", summary.content),
        });
    }
    let after = summary.map_or(0, |value| value.through_sequence.saturating_add(1));
    for event in events.iter().filter(|event| event.sequence >= after) {
        match &event.kind {
            AgentEventKind::UserMessage { content } => history.push(AgentChatMessage {
                role: AgentChatRole::User,
                content: content.clone(),
            }),
            AgentEventKind::AssistantMessage { content, .. } if !content.is_empty() => {
                history.push(AgentChatMessage {
                    role: AgentChatRole::Assistant,
                    content: content.clone(),
                });
            }
            _ => {}
        }
    }
    history
}

async fn refresh_summary<S, F, E>(
    app: &WorkspaceSession<S, F, E>,
    session_id: &AgentSessionId,
    events: &[AgentEvent],
) -> AppResult<Option<AgentSummary>>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    const RECENT_EVENT_COUNT: usize = 30;
    if events.len() <= RECENT_EVENT_COUNT {
        return app.agent.get_summary(session_id).await.map_err(Into::into);
    }
    let through_index = events.len() - RECENT_EVENT_COUNT;
    let summarized = &events[..through_index];
    let mut content = String::new();
    for event in summarized {
        let line = match &event.kind {
            AgentEventKind::UserMessage { content } => format!("User: {content}"),
            AgentEventKind::AssistantMessage { content, .. } => format!("Agent: {content}"),
            AgentEventKind::ToolCall { tool_name, .. } => format!("Tool used: {tool_name}"),
            _ => continue,
        };
        content.push_str(&line);
        content.push('\n');
    }
    if content.len() > 12_000 {
        let keep_from = content
            .char_indices()
            .map(|(index, _)| index)
            .find(|index| content.len().saturating_sub(*index) <= 12_000)
            .unwrap_or(0);
        content = content[keep_from..].to_owned();
    }
    let hash = Sha256::digest(content.as_bytes()).iter().fold(
        String::with_capacity(64),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    );
    let summary = AgentSummary {
        session_id: session_id.clone(),
        through_sequence: summarized.last().map_or(0, |event| event.sequence),
        content,
        content_hash: hash,
        created_at_ms: crate::time::unix_timestamp_ms(),
    };
    app.agent.save_summary(summary.clone()).await?;
    Ok(Some(summary))
}

struct TurnGuard {
    coordinator: Arc<super::AgentTurnCoordinator>,
    session_id: String,
}

impl Drop for TurnGuard {
    fn drop(&mut self) {
        self.coordinator.finish(&self.session_id);
    }
}
