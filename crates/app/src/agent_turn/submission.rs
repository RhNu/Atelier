use super::tools::{AgentTools, agent_app_error};
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentResult, AgentRuntimeEvent};
use atelier_app_api::generation::{
    CharacterDto, GenerateImageRequestDto, GenerationDraftDto, GenerationDraftSeedModeDto,
    GenerationPlanContextDto, GenerationWorkRequestDto, StreamModeDto, SubmitGenerationBatchJobDto,
    SubmitGenerationBatchRequestDto,
};
use atelier_secrets::{ApiKeyId, SecretStore};
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde_json::json;
use std::sync::atomic::Ordering;

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) async fn submit_generation(&self) -> AgentResult<String> {
        if self.submitted.load(Ordering::Acquire) {
            return Err(AgentError::conflict(
                "only one generation batch may be submitted per Agent turn",
            ));
        }
        let current = self.checked_draft().await?;
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
    #[allow(
        clippy::too_many_lines,
        reason = "submission projects one validated draft into the existing generation DTO without duplicating queue logic"
    )]
    pub(super) async fn submit_request(
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

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}
