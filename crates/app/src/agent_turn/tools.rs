use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use async_trait::async_trait;
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{
    AgentAction, AgentActionId, AgentActionState, AgentError, AgentEvent, AgentEventId,
    AgentEventKind, AgentObservation, AgentPermissionMode, AgentResult, AgentRunObserver,
    AgentRuntimeEvent, AgentSessionId, AgentToolExecutor,
};
use atelier_app_api::{
    agent::AgentTurnEventDto,
    generation::{GenerationDraftDto, VersionedGenerationDraftDto},
    prompt::{ListPromptChunksRequestDto, ListPromptPresetsRequestDto, PromptPresetKindDto},
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;

pub use super::schemas::tool_specs;
use super::{AgentTurnCoordinator, draft_edit::DraftEdit};
use crate::{AppError, WorkspaceSession};

pub(super) struct AgentTools<S, F, E> {
    pub(super) app: Arc<WorkspaceSession<S, F, E>>,
    pub(super) session_id: AgentSessionId,
    pub(super) permission_mode: AgentPermissionMode,
    pub(super) coordinator: Arc<AgentTurnCoordinator>,
    pub(super) observer: Arc<dyn AgentRunObserver>,
    pub(super) cancellation: CancellationToken,
    pub(super) next_sequence: AtomicU64,
    pub(super) submitted: AtomicBool,
    draft_observation: Mutex<AgentObservation<VersionedGenerationDraftDto>>,
}

impl<S, F, E> AgentTools<S, F, E> {
    pub fn new(
        app: Arc<WorkspaceSession<S, F, E>>,
        session_id: AgentSessionId,
        permission_mode: AgentPermissionMode,
        observer: Arc<dyn AgentRunObserver>,
        cancellation: CancellationToken,
        next_sequence: u64,
        initial_draft: VersionedGenerationDraftDto,
    ) -> Self {
        Self {
            coordinator: app.agent_turn.clone(),
            app,
            session_id,
            permission_mode,
            observer,
            cancellation,
            next_sequence: AtomicU64::new(next_sequence),
            submitted: AtomicBool::new(false),
            draft_observation: Mutex::new(AgentObservation::new(Some(initial_draft))),
        }
    }
}

#[async_trait]
impl<S, F, E> AgentToolExecutor for AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    async fn begin_model_step(&self) -> AgentResult<()> {
        self.observation()?.begin_model_step();
        Ok(())
    }

    async fn execute(&self, tool_name: &str, arguments_json: &str) -> AgentResult<String> {
        self.append_event(AgentEventKind::ToolCall {
            tool_name: tool_name.to_owned(),
            arguments_json: arguments_json.to_owned(),
        })
        .await?;
        let result = match self.authorize(tool_name, arguments_json).await {
            Ok(()) => self.dispatch(tool_name, arguments_json).await,
            Err(error) => Err(error),
        };
        let (result_json, failed) = match &result {
            Ok(value) => (value.clone(), false),
            Err(error) => (json!({"error": error.to_string()}).to_string(), true),
        };
        self.append_event(AgentEventKind::ToolResult {
            tool_name: tool_name.to_owned(),
            result_json,
            failed,
        })
        .await?;
        result
    }
}

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    async fn authorize(&self, tool_name: &str, arguments_json: &str) -> AgentResult<()> {
        let mutation = !matches!(
            tool_name,
            "get_generation_context" | "list_prompt_chunks" | "list_prompt_presets"
        );
        let needs_approval = mutation
            && match self.permission_mode {
                AgentPermissionMode::Standard => tool_name == "submit_generation",
                AgentPermissionMode::Ask => true,
                AgentPermissionMode::BypassAll => false,
            };
        if !needs_approval {
            return Ok(());
        }
        let approval_id = uuid::Uuid::new_v4().to_string();
        let decision = self
            .coordinator
            .register_approval(self.session_id.as_str(), approval_id.clone())?;
        self.observer.emit(AgentRuntimeEvent::ApprovalRequested {
            approval_id: approval_id.clone(),
            name: tool_name.to_owned(),
            arguments_json: arguments_json.to_owned(),
        });
        let approved = tokio::select! {
            () = self.cancellation.cancelled() => return Err(AgentError::cancelled()),
            decision = decision => decision.unwrap_or(false),
        };
        self.observer.emit(AgentRuntimeEvent::ApprovalResolved {
            approval_id,
            approved,
        });
        self.append_event(AgentEventKind::Approval {
            tool_name: tool_name.to_owned(),
            approved,
        })
        .await?;
        if approved {
            Ok(())
        } else {
            Err(AgentError::conflict("user denied the Agent tool call"))
        }
    }

    async fn dispatch(&self, name: &str, arguments: &str) -> AgentResult<String> {
        match name {
            "get_generation_context" => self.get_context().await,
            "edit_generation_draft" => self.edit_draft(arguments).await,
            "list_prompt_chunks" => self.list_chunks(arguments).await,
            "list_prompt_presets" => self.list_presets(arguments).await,
            "submit_generation" => self.submit_generation().await,
            "undo_agent_action" => self.undo_action(arguments).await,
            _ => Err(AgentError::validation(format!(
                "unknown Agent tool `{name}`"
            ))),
        }
    }

    fn observation(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, AgentObservation<VersionedGenerationDraftDto>>> {
        self.draft_observation
            .lock()
            .map_err(|_| AgentError::runtime("draft observation unavailable"))
    }

    async fn get_context(&self) -> AgentResult<String> {
        let value = self.current_draft().await?;
        let result = json!({"draft": value.draft, "models": atelier_generation::ImageModel::ALL.into_iter().map(crate::mapping::model_descriptor_to_dto).collect::<Vec<_>>()}).to_string();
        self.observation()?.publish(value);
        Ok(result)
    }

    pub(super) async fn checked_draft(&self) -> AgentResult<VersionedGenerationDraftDto> {
        let expected = self.observation()?.known().cloned();
        let current = self.current_draft().await?;
        if expected.as_ref() != Some(&current) {
            let mut observation = self.observation()?;
            observation.invalidate();
            observation.publish(current.clone());
            drop(observation);
            return Err(AgentError::conflict(json!({"code":"outdated", "message":"Review the current draft and replan; queued edits were not applied.", "draft":current.draft}).to_string()));
        }
        Ok(current)
    }

    async fn edit_draft(&self, arguments: &str) -> AgentResult<String> {
        let edit: DraftEdit = parse_args(arguments)?;
        let _guard = self.app.generation_draft_write.lock().await;
        let _resource_guard = self.app.prompt_resource_write.lock().await;
        let before = self.checked_draft().await?;
        let draft = edit.apply(&before.draft)?;
        self.validate_presets(&draft).await?;
        if draft == before.draft {
            return Ok(json!({"changed":false,"draft":draft}).to_string());
        }
        let action_id = AgentActionId::new(uuid::Uuid::new_v4().to_string());
        let action = AgentAction {
            id: action_id.clone(),
            session_id: self.session_id.clone(),
            tool_name: "edit_generation_draft".to_owned(),
            base_revision: before.revision,
            applied_revision: 0,
            before_json: serde_json::to_string(&before.draft).map_err(json_error)?,
            after_json: serde_json::to_string(&draft).map_err(json_error)?,
            state: AgentActionState::Applied,
            created_at_ms: crate::time::unix_timestamp_ms(),
            updated_at_ms: crate::time::unix_timestamp_ms(),
        };
        let saved = self.app.agent_edits.commit_draft(
            before.revision,
            &crate::mapping::generation_draft_to_domain(draft.clone()),
            action,
        )?;
        let after = VersionedGenerationDraftDto {
            revision: saved.revision,
            draft,
        };
        self.observation()?.invalidate();
        self.observation()?.publish(after.clone());
        Ok(json!({"action_id":action_id.as_str(),"changed":true,"draft":after.draft}).to_string())
    }

    async fn validate_presets(&self, draft: &GenerationDraftDto) -> AgentResult<()> {
        let state = draft
            .prompt_states
            .iter()
            .find(|state| state.model == draft.model)
            .ok_or_else(|| AgentError::validation("current model prompt state is missing"))?;
        let presets = self
            .app
            .prompt_presets
            .list_presets(
                None,
                Some(crate::mapping::image_model_to_domain(draft.model)),
            )
            .await
            .map_err(|error| AgentError::validation(error.to_string()))?;
        let mut selected = vec![(
            state.main_preset_id.as_deref(),
            atelier_prompt_resources::PromptPresetKind::Main,
        )];
        selected.extend(state.characters.iter().map(|character| {
            (
                character.preset_id.as_deref(),
                atelier_prompt_resources::PromptPresetKind::Character,
            )
        }));
        for (id, kind) in selected {
            if let Some(id) = id
                && !presets
                    .iter()
                    .any(|preset| preset.id.as_str() == id && preset.kind == kind)
            {
                return Err(AgentError::validation(
                    "selected preset is missing, incompatible, or has the wrong kind",
                ));
            }
        }
        Ok(())
    }

    async fn undo_action(&self, arguments: &str) -> AgentResult<String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Args {
            action_id: String,
        }
        let args: Args = parse_args(arguments)?;
        self.checked_draft().await?;
        let restored = super::runner::undo_in_session(&self.app, &args.action_id)
            .await
            .map_err(agent_app_error)?;
        self.observation()?.invalidate();
        self.observation()?.publish(restored.clone());
        Ok(json!({"undone":true,"draft":restored.draft}).to_string())
    }

    async fn current_draft(&self) -> AgentResult<VersionedGenerationDraftDto> {
        self.app
            .generation()
            .get_draft()
            .await
            .map_err(agent_app_error)?
            .ok_or_else(|| AgentError::not_found("generation draft does not exist"))
    }

    async fn append_event(&self, kind: AgentEventKind) -> AgentResult<()> {
        self.app
            .agent
            .append_event(AgentEvent {
                id: AgentEventId::new(uuid::Uuid::new_v4().to_string()),
                session_id: self.session_id.clone(),
                sequence: self.next_sequence.fetch_add(1, Ordering::AcqRel),
                created_at_ms: crate::time::unix_timestamp_ms(),
                kind,
            })
            .await
    }
    async fn list_chunks(&self, arguments_json: &str) -> AgentResult<String> {
        let args: SearchArgs = parse_args(arguments_json)?;
        let model = self.current_draft().await?.draft.model;
        let mut page = self
            .app
            .prompt()
            .list_chunks(ListPromptChunksRequestDto {
                model: Some(model),
                offset: 0,
                limit: 500,
            })
            .await
            .map_err(agent_app_error)?;
        filter_chunks(&mut page.items, args.query.as_deref());
        page.items.truncate(args.limit.unwrap_or(30).min(100));
        serde_json::to_string(&page.items).map_err(|error| AgentError::runtime(error.to_string()))
    }

    async fn list_presets(&self, arguments_json: &str) -> AgentResult<String> {
        let args: PresetSearchArgs = parse_args(arguments_json)?;
        let model = self.current_draft().await?.draft.model;
        let mut page = self
            .app
            .prompt()
            .list_presets(ListPromptPresetsRequestDto {
                kind: args.kind,
                model: Some(model),
                offset: 0,
                limit: 500,
            })
            .await
            .map_err(agent_app_error)?;
        filter_presets(&mut page.items, args.query.as_deref());
        page.items.truncate(args.limit.unwrap_or(30).min(100));
        serde_json::to_string(&page.items).map_err(|error| AgentError::runtime(error.to_string()))
    }
}
#[derive(Deserialize)]
struct SearchArgs {
    query: Option<String>,
    limit: Option<usize>,
}
#[derive(Deserialize)]
struct PresetSearchArgs {
    query: Option<String>,
    kind: Option<PromptPresetKindDto>,
    limit: Option<usize>,
}

pub(super) fn parse_args<T: for<'de> Deserialize<'de>>(value: &str) -> AgentResult<T> {
    serde_json::from_str(value)
        .map_err(|error| AgentError::validation(format!("invalid tool arguments: {error}")))
}
fn json_error(error: serde_json::Error) -> AgentError {
    let message = error.to_string();
    drop(error);
    AgentError::runtime(message)
}
pub(super) fn agent_app_error(error: AppError) -> AgentError {
    let kind = if error.code().contains("conflict") {
        atelier_agent::AgentErrorKind::Conflict
    } else {
        atelier_agent::AgentErrorKind::Runtime
    };
    let message = error.to_string();
    drop(error);
    AgentError::new(kind, message)
}

fn filter_chunks(items: &mut Vec<atelier_app_api::prompt::PromptChunkDto>, query: Option<&str>) {
    if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
        let query = query.to_lowercase();
        items.retain(|value| {
            format!(
                "{} {} {} {}",
                value.identifier,
                value.display_name,
                value.aliases.join(" "),
                value.content
            )
            .to_lowercase()
            .contains(&query)
        });
    }
}

fn filter_presets(items: &mut Vec<atelier_app_api::prompt::PromptPresetDto>, query: Option<&str>) {
    if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
        let query = query.to_lowercase();
        items.retain(|value| {
            format!(
                "{} {} {}",
                value.identifier,
                value.display_name,
                value.aliases.join(" ")
            )
            .to_lowercase()
            .contains(&query)
        });
    }
}

pub fn runtime_event_to_dto(event: AgentRuntimeEvent) -> AgentTurnEventDto {
    match event {
        AgentRuntimeEvent::AssistantTextDelta { text } => {
            AgentTurnEventDto::AssistantTextDelta { text }
        }
        AgentRuntimeEvent::ToolStarted {
            name,
            arguments_json,
        } => AgentTurnEventDto::ToolStarted {
            name,
            arguments_json,
        },
        AgentRuntimeEvent::ToolFinished {
            name,
            result_json,
            failed: _,
        } if name == "__generation_submitted" => {
            let directive = serde_json::from_str(&result_json)
                .unwrap_or(atelier_app_api::generation::QueueDirectiveDto::Idle);
            AgentTurnEventDto::GenerationSubmitted { directive }
        }
        AgentRuntimeEvent::ToolFinished {
            name,
            result_json,
            failed,
        } => AgentTurnEventDto::ToolFinished {
            name,
            result_json,
            failed,
        },
        AgentRuntimeEvent::ApprovalRequested {
            approval_id,
            name,
            arguments_json,
        } => AgentTurnEventDto::ApprovalRequested {
            approval_id,
            name,
            arguments_json,
        },
        AgentRuntimeEvent::ApprovalResolved {
            approval_id,
            approved,
        } => AgentTurnEventDto::ApprovalResolved {
            approval_id,
            approved,
        },
        AgentRuntimeEvent::Usage {
            input_tokens,
            output_tokens,
        } => AgentTurnEventDto::Usage {
            input_tokens,
            output_tokens,
        },
    }
}
