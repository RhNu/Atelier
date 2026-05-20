use nai_atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GenerationPlanContext, GenerationRequestPlan,
};
use nai_atelier_jobs::{BatchId, JobId, JobPayloadRef};
use nai_atelier_prompt_resources::CompiledPrompt;

#[derive(Clone, Debug, PartialEq)]
pub enum GenerationWorkRequest {
    Image(GenerateImageRequest),
    Stream(GenerateImageStreamRequest),
}

impl GenerationWorkRequest {
    #[must_use]
    pub fn prompt(&self) -> &str {
        match self {
            Self::Image(request) => &request.prompt,
            Self::Stream(request) => &request.base.prompt,
        }
    }

    #[must_use]
    pub fn with_prompt(mut self, prompt: String) -> Self {
        match &mut self {
            Self::Image(request) => request.prompt = prompt,
            Self::Stream(request) => request.base.prompt = prompt,
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubmitGenerationWork {
    pub batch_id: BatchId,
    pub job_id: JobId,
    pub request: GenerationWorkRequest,
    pub context: GenerationPlanContext,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubmittedGenerationPayload {
    pub payload_ref: JobPayloadRef,
    pub batch_id: BatchId,
    pub job_id: JobId,
    pub request: GenerationWorkRequest,
    pub context: GenerationPlanContext,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedGenerationPayload {
    pub payload_ref: JobPayloadRef,
    pub submitted_payload_ref: JobPayloadRef,
    pub batch_id: BatchId,
    pub job_id: JobId,
    pub request: GenerationWorkRequest,
    pub compiled_prompt: CompiledPrompt,
    pub plan: GenerationRequestPlan,
}
