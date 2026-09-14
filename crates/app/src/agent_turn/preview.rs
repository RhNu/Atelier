use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_agent::{AgentError, AgentObservation, AgentResult};
use atelier_app_api::generation::{GenerationPlanContextDto, VersionedGenerationDraftDto};
use atelier_prompt_resources::{PromptChunk, PromptPreset};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use serde_json::json;

use super::{
    submission::{draft_request, subscription_context},
    tools::{AgentTools, agent_app_error},
};

#[derive(Clone, PartialEq, Eq)]
pub struct PromptSources {
    chunks: Vec<PromptChunk>,
    presets: Vec<PromptPreset>,
}

#[derive(Clone)]
pub struct GenerationPreview {
    pub draft: VersionedGenerationDraftDto,
    sources: PromptSources,
    pub prepared: crate::usecases::PreparedTextGeneration,
    pub context: Option<GenerationPlanContextDto>,
    pub result_json: String,
}

impl<S, F, E> AgentTools<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(super) fn preview_observed(
        &self,
    ) -> AgentResult<std::sync::MutexGuard<'_, AgentObservation<GenerationPreview>>> {
        self.preview_observation
            .lock()
            .map_err(|_| AgentError::runtime("generation preview unavailable"))
    }

    async fn prompt_sources(&self) -> AgentResult<PromptSources> {
        let chunks = self
            .app
            .prompt_chunks
            .list_chunks(None)
            .await
            .map_err(|error| AgentError::runtime(error.to_string()))?;
        let presets = self
            .app
            .prompt_presets
            .list_presets(None, None)
            .await
            .map_err(|error| AgentError::runtime(error.to_string()))?;
        Ok(PromptSources { chunks, presets })
    }

    pub(super) async fn preview_generation(&self) -> AgentResult<String> {
        let subscription = self.active_subscription().await;
        let _draft = self.app.generation_draft_write.lock().await;
        let _resources = self.app.prompt_resource_write.lock().await;
        let draft = self.checked_draft().await?;
        let (request, stream) = draft_request(&draft.draft)?;
        let sources = self.prompt_sources().await?;
        let prepared = self
            .app
            .generation()
            .prepare_text_generation(request, stream)
            .await
            .map_err(agent_app_error)?;
        let resolved_prompts = prepared.resolved_prompts().map_err(agent_app_error)?;
        let usage = prepared.token_usage().map_err(agent_app_error)?;
        let context = subscription
            .as_ref()
            .ok()
            .map(|value| subscription_context(value, draft.draft.request_count));
        let estimate = context
            .map(|value| prepared.estimate(value))
            .transpose()
            .map_err(agent_app_error)?;
        let result_json = json!({
            "request":prepared.request(),"stream":stream,"request_count":draft.draft.request_count,
            "resolved_prompts":resolved_prompts,"prompt_trace":prepared.trace(),"token_usage":usage,"anlas_estimate":estimate,
            "estimate_unavailable_reason":subscription.err().map(|error| error.to_string()),
            "seed_mode":draft.draft.seed_mode,
            "submission":"Submit this observed compiled snapshot. Any draft or prompt-library change requires a new preview. Quality/UC fields are applied by NovelAI request assembly; token counts include them."
        }).to_string();
        self.preview_observed()?.publish(GenerationPreview {
            draft,
            sources,
            prepared,
            context,
            result_json: result_json.clone(),
        });
        Ok(result_json)
    }

    /// Called with both draft and prompt writer locks held, including after approval.
    pub(super) async fn checked_preview(&self) -> AgentResult<GenerationPreview> {
        let preview = self.preview_observed()?.known().cloned().ok_or_else(|| AgentError::conflict("call preview_generation and review its result in the next model response before submitting"))?;
        let draft = self.checked_draft().await?;
        if preview.draft != draft || preview.sources != self.prompt_sources().await? {
            self.preview_observed()?.invalidate();
            return Err(AgentError::conflict(json!({"code":"outdated","message":"The draft or prompt library changed. Call preview_generation again and review the new result before submitting."}).to_string()));
        }
        Ok(preview)
    }

    pub(super) async fn submission_approval(&self) -> AgentResult<String> {
        let _draft = self.app.generation_draft_write.lock().await;
        let _resources = self.app.prompt_resource_write.lock().await;
        Ok(self.checked_preview().await?.result_json)
    }
}
