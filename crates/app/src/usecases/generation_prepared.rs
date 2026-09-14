use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{
    GenerateImageRequestDto, GenerationAnlasEstimateDto, GenerationEstimateRequestDto,
    GenerationPlanContextDto, PromptTokenUsageDto, QueueDirectiveDto,
};
use atelier_generation::{GenerateImageRequest, GenerateImageStreamRequest, StreamMode};
use atelier_jobs::{BatchId, JobId};
use atelier_kernel::{GenerationWorkRequest, SubmitGenerationBatch, SubmitGenerationBatchJob};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use super::{
    GenerationUseCases, generation_support::estimate_generation_anlas,
    generation_tokens::prompt_token_usage_to_dto, history::GenerationHistoryPosition,
};
use crate::{AppError, AppResult};

/// An immutable compiled text request. Submission never invokes prompt compilation again.
#[derive(Clone)]
pub struct PreparedTextGeneration {
    request: GenerateImageRequestDto,
    domain: GenerateImageRequest,
    prompt: atelier_prompt_resources::CompiledPrompt,
    stream: bool,
    trace: PreparedPromptTrace,
}

impl PreparedTextGeneration {
    fn from_compiled(
        mut request: GenerateImageRequestDto,
        domain: GenerateImageRequest,
        prompt: atelier_prompt_resources::CompiledPrompt,
        trace: PreparedPromptTrace,
        stream: bool,
    ) -> AppResult<Self> {
        let domain = atelier_generation::normalize_generate_request(domain)
            .map_err(|error| AppError::new("invalid_request", error.to_string()))?;
        request.size.width = domain.size.width;
        request.size.height = domain.size.height;
        request.steps = domain.steps;
        request.scale = domain.scale;
        request.n_samples = domain.n_samples;
        request.cfg_rescale = domain.cfg_rescale;
        request.quality = crate::mapping::quality_preset_to_dto(domain.quality);
        request.furry_mode = domain.furry_mode;
        request.variety_boost = domain.variety_boost;
        request.transparent_background = domain.transparent_background;
        request.characters = domain.characters.as_ref().map(|characters| {
            characters
                .iter()
                .map(|value| atelier_app_api::generation::CharacterDto {
                    preset_id: None,
                    prompt: value.prompt.clone(),
                    negative_prompt: value.negative_prompt.clone(),
                    position: atelier_app_api::generation::CharacterPositionDto {
                        x: value.position.x,
                        y: value.position.y,
                    },
                    enabled: value.enabled,
                })
                .collect()
        });
        Ok(Self {
            request,
            domain,
            prompt,
            stream,
            trace,
        })
    }

    pub const fn request(&self) -> &GenerateImageRequestDto {
        &self.request
    }

    pub fn token_usage(&self) -> AppResult<PromptTokenUsageDto> {
        atelier_generation::count_prompt_tokens(&self.domain)
            .map(prompt_token_usage_to_dto)
            .map_err(|error| AppError::new("prompt_tokenizer", error.to_string()))
    }

    pub fn estimate(
        &self,
        context: GenerationPlanContextDto,
    ) -> AppResult<GenerationAnlasEstimateDto> {
        estimate_generation_anlas(&GenerationEstimateRequestDto {
            request: self.request.clone(),
            context,
        })
    }

    pub fn resolved_prompts(&self) -> AppResult<serde_json::Value> {
        let prompts = atelier_generation::resolve_prompt_text(&self.domain)
            .map_err(|error| AppError::new("prompt_resolution", error.to_string()))?;
        Ok(serde_json::json!(prompts))
    }

    pub const fn trace(&self) -> &PreparedPromptTrace {
        &self.trace
    }

    fn work(&self) -> GenerationWorkRequest {
        if self.stream {
            GenerationWorkRequest::Stream(GenerateImageStreamRequest {
                base: self.domain.clone(),
                stream: StreamMode::Sse,
            })
        } else {
            GenerationWorkRequest::Image(self.domain.clone())
        }
    }
}

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn prepare_text_generation(
        &self,
        request: GenerateImageRequestDto,
        stream: bool,
    ) -> AppResult<PreparedTextGeneration> {
        if request.img2img.is_some()
            || request.vibe_transfer.is_some()
            || request.character_references.is_some()
        {
            return Err(AppError::new(
                "invalid_request",
                "prepared Agent generation accepts text only",
            ));
        }
        let (request, prompt, trace) = self.prepare_prompt_details(request).await?;
        let domain = self.generate_request_to_domain(request.clone()).await?;
        PreparedTextGeneration::from_compiled(request, domain, prompt, trace, stream)
    }

    pub async fn submit_prepared_text(
        &self,
        prepared: &PreparedTextGeneration,
        batch_id: String,
        job_ids: Vec<String>,
        context: GenerationPlanContextDto,
    ) -> AppResult<QueueDirectiveDto> {
        let history = job_ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                (
                    id.clone(),
                    crate::mapping::generation_work_title(
                        &atelier_app_api::generation::GenerationWorkRequestDto::Image(
                            prepared.request.clone(),
                        ),
                    ),
                    GenerationHistoryPosition {
                        request_index: u32::try_from(index).unwrap_or(u32::MAX),
                        expected_samples: prepared.request.n_samples,
                    },
                )
            })
            .collect();
        let jobs = job_ids
            .into_iter()
            .map(|id| SubmitGenerationBatchJob {
                compiled_prompt: Some(prepared.prompt.clone()),
                job_id: JobId::new(id),
                request: prepared.work(),
            })
            .collect();
        let work = SubmitGenerationBatch {
            batch_id: BatchId::new(&batch_id),
            jobs,
            context: crate::mapping::plan_context_to_domain(context),
        };
        self.submit_ready_batch(batch_id, history, work).await
    }
}

#[derive(Clone, Default, serde::Serialize)]
pub struct PreparedPromptTrace {
    main: Option<atelier_app_api::prompt::PromptTraceDto>,
    negative: Option<atelier_app_api::prompt::PromptTraceDto>,
    preset_ids: Vec<String>,
    characters: Vec<PreparedCharacterTrace>,
}

#[cfg(test)]
mod tests;

#[derive(Clone, serde::Serialize)]
struct PreparedCharacterTrace {
    index: u32,
    prompt: atelier_app_api::prompt::PromptTraceDto,
    negative: atelier_app_api::prompt::PromptTraceDto,
}

impl From<&atelier_prompt_resources::CompiledGenerationPrompt> for PreparedPromptTrace {
    fn from(value: &atelier_prompt_resources::CompiledGenerationPrompt) -> Self {
        Self {
            main: value
                .trace
                .main_prompt
                .as_ref()
                .map(crate::mapping::prompt_trace_to_dto),
            negative: value
                .trace
                .main_negative_prompt
                .as_ref()
                .map(crate::mapping::prompt_trace_to_dto),
            preset_ids: value
                .trace
                .used_presets
                .iter()
                .map(|value| value.preset_id.as_str().to_owned())
                .collect(),
            characters: value
                .characters
                .iter()
                .map(|value| PreparedCharacterTrace {
                    index: value.character_index,
                    prompt: crate::mapping::prompt_trace_to_dto(&value.trace),
                    negative: crate::mapping::prompt_trace_to_dto(&value.negative_trace),
                })
                .collect(),
        }
    }
}
