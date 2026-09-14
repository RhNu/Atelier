use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use async_trait::async_trait;
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{
    AgentAction, AgentActionId, AgentActionState, AgentError, AgentEvent, AgentEventId,
    AgentEventKind, AgentPermissionMode, AgentResult, AgentRunObserver, AgentRuntimeEvent,
    AgentSessionId, AgentToolExecutor, AgentToolSpec,
};
use atelier_app_api::{
    agent::AgentTurnEventDto,
    generation::{
        CharacterDto, GenerateImageRequestDto, GenerationDraftDto, GenerationDraftSeedModeDto,
        GenerationPlanContextDto, GenerationWorkRequestDto, StreamModeDto,
        SubmitGenerationBatchJobDto, SubmitGenerationBatchRequestDto, VersionedGenerationDraftDto,
    },
    prompt::{
        GetPromptChunkRequestDto, ListPromptChunksRequestDto, ListPromptPresetsRequestDto,
        PromptPresetKindDto,
    },
};
use atelier_secrets::{ApiKeyId, SecretStore};
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::AgentTurnCoordinator;
use crate::{AppError, WorkspaceSession};

pub struct AgentTools<S, F, E> {
    app: Arc<WorkspaceSession<S, F, E>>,
    session_id: AgentSessionId,
    permission_mode: AgentPermissionMode,
    coordinator: Arc<AgentTurnCoordinator>,
    observer: Arc<dyn AgentRunObserver>,
    cancellation: CancellationToken,
    next_sequence: AtomicU64,
    submitted: AtomicBool,
}

impl<S, F, E> AgentTools<S, F, E> {
    pub fn new(
        app: Arc<WorkspaceSession<S, F, E>>,
        session_id: AgentSessionId,
        permission_mode: AgentPermissionMode,
        coordinator: Arc<AgentTurnCoordinator>,
        observer: Arc<dyn AgentRunObserver>,
        cancellation: CancellationToken,
        next_sequence: u64,
    ) -> Self {
        Self {
            app,
            session_id,
            permission_mode,
            coordinator,
            observer,
            cancellation,
            next_sequence: AtomicU64::new(next_sequence),
            submitted: AtomicBool::new(false),
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
    async fn execute(&self, tool_name: &str, arguments_json: &str) -> AgentResult<String> {
        self.append_event(AgentEventKind::ToolCall {
            tool_name: tool_name.to_owned(),
            arguments_json: arguments_json.to_owned(),
        })
        .await?;
        self.authorize(tool_name, arguments_json).await?;
        let result = self.dispatch(tool_name, arguments_json).await;
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
            "get_generation_draft" | "list_prompt_chunks" | "list_prompt_presets"
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

    async fn dispatch(&self, tool_name: &str, arguments_json: &str) -> AgentResult<String> {
        match tool_name {
            "get_generation_draft" => self.get_draft().await,
            "list_prompt_chunks" => self.list_chunks(arguments_json).await,
            "list_prompt_presets" => self.list_presets(arguments_json).await,
            "update_main_prompt" => self.update_main_prompt(arguments_json).await,
            "upsert_character" => self.upsert_character(arguments_json).await,
            "remove_character" => self.remove_character(arguments_json).await,
            "set_image_size" => self.set_image_size(arguments_json).await,
            "set_generation_parameters" => self.set_parameters(arguments_json).await,
            "apply_prompt_chunk" => self.apply_chunk(arguments_json).await,
            "apply_prompt_preset" => self.apply_preset(arguments_json).await,
            "submit_generation" => self.submit_generation(arguments_json).await,
            _ => Err(AgentError::validation(format!(
                "unknown Agent tool `{tool_name}`"
            ))),
        }
    }

    async fn get_draft(&self) -> AgentResult<String> {
        let value = self
            .app
            .generation()
            .get_draft()
            .await
            .map_err(agent_app_error)?;
        serde_json::to_string(&value).map_err(|error| AgentError::runtime(error.to_string()))
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

    async fn update_main_prompt(&self, arguments_json: &str) -> AgentResult<String> {
        let args: UpdateMainPromptArgs = parse_args(arguments_json)?;
        self.mutate("update_main_prompt", args.expected_revision, move |draft| {
            let state = active_prompt_state_mut(draft)?;
            if let Some(prompt) = args.prompt {
                state.prompt = prompt;
            }
            if let Some(negative_prompt) = args.negative_prompt {
                state.negative_prompt = negative_prompt;
            }
            Ok(())
        })
        .await
    }

    async fn upsert_character(&self, arguments_json: &str) -> AgentResult<String> {
        let args: UpsertCharacterArgs = parse_args(arguments_json)?;
        self.mutate("upsert_character", args.expected_revision, move |draft| {
            let state = active_prompt_state_mut(draft)?;
            let id = args
                .id
                .unwrap_or_else(|| format!("character-{}", uuid::Uuid::new_v4()));
            let character = atelier_app_api::generation::GenerationDraftCharacterDto {
                id: id.clone(),
                preset_id: args.preset_id,
                prompt: args.prompt,
                negative_prompt: args.negative_prompt.unwrap_or_default(),
                enabled: args.enabled.unwrap_or(true),
                position: atelier_app_api::generation::CharacterPositionDto {
                    x: args.x.unwrap_or(0.5),
                    y: args.y.unwrap_or(0.5),
                },
            };
            if let Some(current) = state.characters.iter_mut().find(|value| value.id == id) {
                *current = character;
            } else {
                state.characters.push(character);
            }
            Ok(())
        })
        .await
    }

    async fn remove_character(&self, arguments_json: &str) -> AgentResult<String> {
        let args: CharacterIdArgs = parse_args(arguments_json)?;
        self.mutate("remove_character", args.expected_revision, move |draft| {
            let state = active_prompt_state_mut(draft)?;
            let before = state.characters.len();
            state
                .characters
                .retain(|value| value.id != args.character_id);
            if before == state.characters.len() {
                return Err(AgentError::not_found("generation character does not exist"));
            }
            Ok(())
        })
        .await
    }

    async fn set_image_size(&self, arguments_json: &str) -> AgentResult<String> {
        let args: SizeArgs = parse_args(arguments_json)?;
        let (width, height) = size_preset(&args.preset)?;
        self.mutate("set_image_size", args.expected_revision, move |draft| {
            draft.size.width = width;
            draft.size.height = height;
            Ok(())
        })
        .await
    }

    async fn set_parameters(&self, arguments_json: &str) -> AgentResult<String> {
        let args: ParameterArgs = parse_args(arguments_json)?;
        self.mutate(
            "set_generation_parameters",
            args.expected_revision,
            move |draft| {
                if let Some(value) = args.steps {
                    draft.steps = value;
                }
                if let Some(value) = args.scale {
                    draft.scale = value;
                }
                if let Some(value) = args.n_samples {
                    draft.n_samples = value;
                }
                if let Some(value) = args.request_count {
                    draft.request_count = value;
                }
                if let Some(value) = args.cfg_rescale {
                    draft.cfg_rescale = value;
                }
                if let Some(value) = args.variety_boost {
                    draft.variety_boost = value;
                }
                if let Some(value) = args.seed {
                    draft.seed = value;
                    draft.seed_mode = if value == 0 {
                        GenerationDraftSeedModeDto::Random
                    } else {
                        GenerationDraftSeedModeDto::Fixed
                    };
                }
                Ok(())
            },
        )
        .await
    }

    async fn apply_chunk(&self, arguments_json: &str) -> AgentResult<String> {
        let args: ApplyChunkArgs = parse_args(arguments_json)?;
        let chunk = self
            .app
            .prompt()
            .get_chunk(GetPromptChunkRequestDto {
                chunk_id: Some(args.chunk_id),
                path: None,
            })
            .await
            .map_err(agent_app_error)?;
        let expression = format!("$chunk({})", chunk.identifier);
        self.mutate("apply_prompt_chunk", args.expected_revision, move |draft| {
            let prompt = prompt_target_mut(draft, &args.target, args.character_id.as_deref())?;
            append_prompt_fragment(prompt, &expression);
            Ok(())
        })
        .await
    }

    async fn apply_preset(&self, arguments_json: &str) -> AgentResult<String> {
        let args: ApplyPresetArgs = parse_args(arguments_json)?;
        let kind = if args.character_id.is_some() {
            PromptPresetKindDto::Character
        } else {
            PromptPresetKindDto::Main
        };
        let model = self.current_draft().await?.draft.model;
        let page = self
            .app
            .prompt()
            .list_presets(ListPromptPresetsRequestDto {
                kind: Some(kind),
                model: Some(model),
                offset: 0,
                limit: 500,
            })
            .await
            .map_err(agent_app_error)?;
        if !page
            .items
            .iter()
            .any(|value| value.preset_id == args.preset_id)
        {
            return Err(AgentError::not_found(
                "prompt preset does not exist for this model",
            ));
        }
        self.mutate(
            "apply_prompt_preset",
            args.expected_revision,
            move |draft| {
                let state = active_prompt_state_mut(draft)?;
                if let Some(character_id) = args.character_id {
                    let character = state
                        .characters
                        .iter_mut()
                        .find(|value| value.id == character_id)
                        .ok_or_else(|| {
                            AgentError::not_found("generation character does not exist")
                        })?;
                    character.preset_id = Some(args.preset_id);
                } else {
                    state.main_preset_id = Some(args.preset_id);
                }
                Ok(())
            },
        )
        .await
    }

    async fn submit_generation(&self, arguments_json: &str) -> AgentResult<String> {
        let args: ExpectedRevisionArgs = parse_args(arguments_json)?;
        if self.submitted.load(Ordering::Acquire) {
            return Err(AgentError::conflict(
                "only one generation batch may be submitted per Agent turn",
            ));
        }
        let current = self.current_draft().await?;
        if current.revision != args.expected_revision {
            return Err(AgentError::conflict("generation draft revision changed"));
        }
        let request = self.submit_request(&current.draft).await?;
        let directive = self
            .app
            .generation()
            .submit_batch(request)
            .await
            .map_err(agent_app_error)?;
        self.submitted.store(true, Ordering::Release);
        self.observer.emit(AgentRuntimeEvent::ToolFinished {
            name: "__generation_submitted".to_owned(),
            result_json: serde_json::to_string(&directive).unwrap_or_default(),
            failed: false,
        });
        Ok(json!({"submitted": true, "directive": directive}).to_string())
    }

    async fn mutate(
        &self,
        tool_name: &str,
        expected_revision: u64,
        edit: impl FnOnce(&mut GenerationDraftDto) -> AgentResult<()>,
    ) -> AgentResult<String> {
        let before = self.current_draft().await?;
        if before.revision != expected_revision {
            return Err(AgentError::conflict(format!(
                "generation draft is at revision {}, not {expected_revision}",
                before.revision
            )));
        }
        let mut draft = before.draft.clone();
        edit(&mut draft)?;
        let after = self
            .app
            .generation()
            .save_draft(atelier_app_api::generation::SaveGenerationDraftRequestDto {
                expected_revision,
                draft,
            })
            .await
            .map_err(agent_app_error)?;
        let action_id = AgentActionId::new(uuid::Uuid::new_v4().to_string());
        let action = AgentAction {
            id: action_id.clone(),
            session_id: self.session_id.clone(),
            tool_name: tool_name.to_owned(),
            base_revision: before.revision,
            applied_revision: after.revision,
            before_json: serde_json::to_string(&before.draft)
                .map_err(|error| AgentError::runtime(error.to_string()))?,
            after_json: serde_json::to_string(&after.draft)
                .map_err(|error| AgentError::runtime(error.to_string()))?,
            state: AgentActionState::Applied,
            created_at_ms: crate::time::unix_timestamp_ms(),
            updated_at_ms: crate::time::unix_timestamp_ms(),
        };
        self.app.agent.save_action(action).await?;
        Ok(json!({
            "action_id": action_id.as_str(),
            "revision": after.revision,
            "draft": after.draft
        })
        .to_string())
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

    #[allow(
        clippy::too_many_lines,
        reason = "submission projects one validated draft into the existing generation DTO without duplicating queue logic"
    )]
    async fn submit_request(
        &self,
        draft: &GenerationDraftDto,
    ) -> AgentResult<SubmitGenerationBatchRequestDto> {
        if draft.i2i.is_some()
            || !draft.vibe.slots.is_empty()
            || !draft.precise_references.is_empty()
        {
            return Err(AgentError::validation(
                "Agent submission is text-only; submit image references from the Generation page",
            ));
        }
        let state = draft
            .prompt_states
            .iter()
            .find(|value| value.model == draft.model)
            .ok_or_else(|| AgentError::validation("current model prompt state is missing"))?;
        if state.prompt.trim().is_empty() && state.main_preset_id.is_none() {
            return Err(AgentError::validation("generation prompt is empty"));
        }
        let capabilities = crate::mapping::model_descriptor_to_dto(
            crate::mapping::image_model_to_domain(draft.model),
        )
        .capabilities;
        let characters = (capabilities.max_characters > 0).then(|| {
            state
                .characters
                .iter()
                .map(|value| CharacterDto {
                    preset_id: value.preset_id.clone(),
                    prompt: value.prompt.clone(),
                    negative_prompt: nonempty(&value.negative_prompt),
                    position: value.position,
                    enabled: value.enabled,
                })
                .collect::<Vec<_>>()
        });
        let base = GenerateImageRequestDto {
            main_preset_id: state.main_preset_id.clone(),
            prompt: state.prompt.clone(),
            furry_mode: capabilities.supports_furry_mode && state.furry_mode,
            model: draft.model,
            size: draft.size,
            negative_prompt: nonempty(&state.negative_prompt),
            quality: if draft.quality == atelier_app_api::generation::QualityPresetDto::Light
                && !capabilities.supports_light_quality_preset
            {
                atelier_app_api::generation::QualityPresetDto::Standard
            } else {
                draft.quality
            },
            transparent_background: capabilities.supports_transparent_background
                && draft.transparent_background,
            uc_preset: draft.uc_preset,
            steps: draft.steps,
            scale: draft.scale,
            sampler: draft.sampler,
            noise_schedule: draft.noise_schedule,
            seed: if draft.seed_mode == GenerationDraftSeedModeDto::Random {
                0
            } else {
                draft.seed
            },
            n_samples: draft.n_samples,
            cfg_rescale: draft.cfg_rescale,
            variety_boost: capabilities.supports_variety_boost && draft.variety_boost,
            strict_mode: draft.strict_mode,
            img2img: None,
            vibe_transfer: None,
            character_references: None,
            characters,
            use_coords: capabilities.character_position_mode.is_some().then_some(
                state.character_position_mode
                    == atelier_app_api::generation::GenerationDraftCharacterPositionModeDto::Manual,
            ),
            image_format: draft.image_format,
        };
        let work = if draft.stream_enabled && capabilities.supports_streaming {
            GenerationWorkRequestDto::Stream(
                atelier_app_api::generation::GenerateImageStreamRequestDto {
                    base,
                    stream: StreamModeDto::Sse,
                },
            )
        } else {
            GenerationWorkRequestDto::Image(base)
        };
        let subscription = self.active_subscription().await?;
        let request_count = draft.request_count.clamp(1, 8);
        Ok(SubmitGenerationBatchRequestDto {
            batch_id: format!("generation-{}", uuid::Uuid::new_v4()),
            jobs: (0..request_count)
                .map(|_| SubmitGenerationBatchJobDto {
                    job_id: format!("job-{}", uuid::Uuid::new_v4()),
                    work: work.clone(),
                })
                .collect(),
            context: GenerationPlanContextDto {
                request_count,
                pending_vibe_encode_count: 0,
                tier: subscription.tier,
                subscription_active: subscription.subscription_active,
                v5_usage_is_negative: subscription.v5_usage.is_some_and(|value| value.is_negative),
            },
        })
    }

    async fn active_subscription(&self) -> AgentResult<atelier_secrets::SubscriptionSummary> {
        let active = self
            .app
            .api_keys
            .list_api_keys()
            .await
            .map_err(|error| AgentError::runtime(error.to_string()))?
            .into_iter()
            .find(|value| value.is_active)
            .ok_or_else(|| AgentError::runtime("no active NovelAI API key"))?;
        self.app
            .api_keys
            .probe_key(&ApiKeyId::new(active.id.as_str()))
            .await
            .map_err(|error| AgentError::runtime(error.to_string()))
    }
}

pub fn tool_specs() -> Vec<AgentToolSpec> {
    vec![
        spec(
            "get_generation_draft",
            "Read the current versioned generation draft.",
            json!({"type":"object","properties":{}}),
        ),
        spec(
            "list_prompt_chunks",
            "Search prompt chunks available for the current NovelAI model.",
            search_schema(),
        ),
        spec(
            "list_prompt_presets",
            "Search main or character prompt presets available for the current NovelAI model.",
            json!({"type":"object","properties":{"query":{"type":"string"},"kind":{"type":"string","enum":["main","character"]},"limit":{"type":"integer","minimum":1,"maximum":100}}}),
        ),
        spec(
            "update_main_prompt",
            "Replace the current model's main positive and/or negative prompt.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"prompt":{"type":"string"},"negative_prompt":{"type":"string"}},"required":["expected_revision"]}),
        ),
        spec(
            "upsert_character",
            "Create or replace one character prompt, negative prompt, enabled state, preset, and normalized position.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"id":{"type":"string"},"preset_id":{"type":"string"},"prompt":{"type":"string"},"negative_prompt":{"type":"string"},"enabled":{"type":"boolean"},"x":{"type":"number","minimum":0,"maximum":1},"y":{"type":"number","minimum":0,"maximum":1}},"required":["expected_revision","prompt"]}),
        ),
        spec(
            "remove_character",
            "Remove a character prompt by stable id.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"character_id":{"type":"string"}},"required":["expected_revision","character_id"]}),
        ),
        spec(
            "set_image_size",
            "Set a named image size preset.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"preset":{"type":"string","enum":["normal_portrait","normal_landscape","normal_square","large_portrait","large_landscape","small_portrait","small_landscape","small_square"]}},"required":["expected_revision","preset"]}),
        ),
        spec(
            "set_generation_parameters",
            "Update numeric generation controls and seed behavior.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"steps":{"type":"integer"},"scale":{"type":"number"},"seed":{"type":"integer"},"n_samples":{"type":"integer"},"request_count":{"type":"integer"},"cfg_rescale":{"type":"number"},"variety_boost":{"type":"boolean"}},"required":["expected_revision"]}),
        ),
        spec(
            "apply_prompt_chunk",
            "Append a prompt chunk reference to a main or character prompt.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"chunk_id":{"type":"string"},"target":{"type":"string","enum":["main_positive","main_negative","character_positive","character_negative"]},"character_id":{"type":"string"}},"required":["expected_revision","chunk_id","target"]}),
        ),
        spec(
            "apply_prompt_preset",
            "Select a compatible main or character prompt preset.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"},"preset_id":{"type":"string"},"character_id":{"type":"string"}},"required":["expected_revision","preset_id"]}),
        ),
        spec(
            "submit_generation",
            "Submit the current text-only draft as one generation batch. This is limited to one call per user turn.",
            json!({"type":"object","properties":{"expected_revision":{"type":"integer"}},"required":["expected_revision"]}),
        ),
    ]
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "tool schemas are short-lived values serialized at this boundary"
)]
fn spec(name: &str, description: &str, schema: Value) -> AgentToolSpec {
    AgentToolSpec {
        name: name.to_owned(),
        description: description.to_owned(),
        parameters_json: schema.to_string(),
    }
}

fn search_schema() -> Value {
    json!({"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":100}}})
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
#[derive(Deserialize)]
struct ExpectedRevisionArgs {
    expected_revision: u64,
}
#[derive(Deserialize)]
struct UpdateMainPromptArgs {
    expected_revision: u64,
    prompt: Option<String>,
    negative_prompt: Option<String>,
}
#[derive(Deserialize)]
struct CharacterIdArgs {
    expected_revision: u64,
    character_id: String,
}
#[derive(Deserialize)]
struct SizeArgs {
    expected_revision: u64,
    preset: String,
}
#[derive(Deserialize)]
struct ApplyChunkArgs {
    expected_revision: u64,
    chunk_id: String,
    target: String,
    character_id: Option<String>,
}
#[derive(Deserialize)]
struct ApplyPresetArgs {
    expected_revision: u64,
    preset_id: String,
    character_id: Option<String>,
}
#[derive(Deserialize)]
struct UpsertCharacterArgs {
    expected_revision: u64,
    id: Option<String>,
    preset_id: Option<String>,
    prompt: String,
    negative_prompt: Option<String>,
    enabled: Option<bool>,
    x: Option<f32>,
    y: Option<f32>,
}
#[derive(Deserialize)]
struct ParameterArgs {
    expected_revision: u64,
    steps: Option<u32>,
    scale: Option<f32>,
    seed: Option<i64>,
    n_samples: Option<u32>,
    request_count: Option<u32>,
    cfg_rescale: Option<f32>,
    variety_boost: Option<bool>,
}

fn parse_args<T: for<'de> Deserialize<'de>>(value: &str) -> AgentResult<T> {
    serde_json::from_str(value)
        .map_err(|error| AgentError::validation(format!("invalid tool arguments: {error}")))
}

fn active_prompt_state_mut(
    draft: &mut GenerationDraftDto,
) -> AgentResult<&mut atelier_app_api::generation::GenerationDraftPromptStateDto> {
    draft
        .prompt_states
        .iter_mut()
        .find(|value| value.model == draft.model)
        .ok_or_else(|| AgentError::validation("current model prompt state is missing"))
}

fn prompt_target_mut<'a>(
    draft: &'a mut GenerationDraftDto,
    target: &str,
    character_id: Option<&str>,
) -> AgentResult<&'a mut String> {
    let state = active_prompt_state_mut(draft)?;
    match target {
        "main_positive" => Ok(&mut state.prompt),
        "main_negative" => Ok(&mut state.negative_prompt),
        "character_positive" | "character_negative" => {
            let id = character_id.ok_or_else(|| {
                AgentError::validation("character_id is required for a character target")
            })?;
            let character = state
                .characters
                .iter_mut()
                .find(|value| value.id == id)
                .ok_or_else(|| AgentError::not_found("generation character does not exist"))?;
            if target == "character_positive" {
                Ok(&mut character.prompt)
            } else {
                Ok(&mut character.negative_prompt)
            }
        }
        _ => Err(AgentError::validation("unknown prompt target")),
    }
}

fn append_prompt_fragment(prompt: &mut String, value: &str) {
    if !prompt.trim().is_empty() {
        prompt.push_str(", ");
    }
    prompt.push_str(value);
}

fn size_preset(value: &str) -> AgentResult<(u32, u32)> {
    match value {
        "normal_portrait" => Ok((832, 1216)),
        "normal_landscape" => Ok((1216, 832)),
        "normal_square" => Ok((1024, 1024)),
        "large_portrait" => Ok((1024, 1536)),
        "large_landscape" => Ok((1536, 1024)),
        "small_portrait" => Ok((512, 768)),
        "small_landscape" => Ok((768, 512)),
        "small_square" => Ok((640, 640)),
        _ => Err(AgentError::validation("unknown image size preset")),
    }
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

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

#[allow(
    clippy::needless_pass_by_value,
    reason = "map_err supplies owned application errors"
)]
fn agent_app_error(error: AppError) -> AgentError {
    AgentError::runtime(error.to_string())
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
