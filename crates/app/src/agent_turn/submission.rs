use super::tools::{AgentTools, agent_app_error};
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentResult, AgentRuntimeEvent};
use atelier_app_api::generation::{
    CharacterDto, GenerateImageRequestDto, GenerationDraftDto, GenerationDraftSeedModeDto,
    GenerationPlanContextDto,
};
use atelier_secrets::{ApiKeyId, SecretStore};
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde_json::json;

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) async fn submit_generation(&self) -> AgentResult<String> {
        let subscription = self.active_subscription().await?;
        let _draft = self.app.generation_draft_write.lock().await;
        let _resources = self.app.prompt_resource_write.lock().await;
        let preview = self.checked_preview().await?;
        let context = subscription_context(&subscription, preview.draft.draft.request_count);
        if preview.context != Some(context) {
            return Err(AgentError::conflict(
                "subscription context changed or was unavailable; call preview_generation again before submitting",
            ));
        }
        if self.cancellation.is_cancelled() {
            return Err(AgentError::cancelled());
        }
        let batch_id = format!("generation-{}", uuid::Uuid::new_v4());
        let job_ids = (0..context.request_count)
            .map(|_| format!("job-{}", uuid::Uuid::new_v4()))
            .collect::<Vec<_>>();
        self.owned_batches
            .lock()
            .map_err(|_| AgentError::runtime("generation ownership unavailable"))?
            .insert(batch_id.clone());
        self.coordinator.generations.register(
            batch_id.clone(),
            job_ids.clone(),
            &self.cancellation,
        )?;
        let directive = self
            .app
            .generation()
            .submit_prepared_text(
                &preview.prepared,
                batch_id.clone(),
                job_ids.clone(),
                context,
            )
            .await
            .map_err(agent_app_error)?;
        self.generation_batches
            .lock()
            .map_err(|_| AgentError::runtime("generation ownership unavailable"))?
            .insert(batch_id.clone());
        self.preview_observed()?.invalidate();
        self.observer.emit(AgentRuntimeEvent::ToolFinished {
            name: "__generation_submitted".to_owned(),
            result_json: serde_json::to_string(&directive).unwrap_or_default(),
            failed: false,
        });
        Ok(
            json!({"submitted":true,"batch_id":batch_id,"job_ids":job_ids,"directive":directive})
                .to_string(),
        )
    }

    pub(super) async fn active_subscription(
        &self,
    ) -> AgentResult<atelier_secrets::SubscriptionSummary> {
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

#[allow(
    clippy::too_many_lines,
    reason = "projects all supported text-only draft settings into one request"
)]
pub(super) fn draft_request(
    draft: &GenerationDraftDto,
) -> AgentResult<(GenerateImageRequestDto, bool)> {
    if draft.i2i.is_some() || !draft.vibe.slots.is_empty() || !draft.precise_references.is_empty() {
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
    let capabilities =
        crate::mapping::model_descriptor_to_dto(crate::mapping::image_model_to_domain(draft.model))
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
    Ok((
        base,
        draft.stream_enabled && capabilities.supports_streaming,
    ))
}

pub(super) fn subscription_context(
    subscription: &atelier_secrets::SubscriptionSummary,
    request_count: u32,
) -> GenerationPlanContextDto {
    GenerationPlanContextDto {
        request_count,
        pending_vibe_encode_count: 0,
        tier: subscription.tier,
        subscription_active: subscription.subscription_active,
        v5_usage_is_negative: subscription.v5_usage.is_some_and(|value| value.is_negative),
    }
}

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}
