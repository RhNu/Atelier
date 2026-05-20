use nai_atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GenerationPlanContext, GenerationRequestPlan,
};
use nai_atelier_jobs::{BatchId, JobId, JobPayloadRef};
use nai_atelier_prompt_resources::CompiledPrompt;
use nai_atelier_vibe::{
    VibeDocumentEntry, VibeEncodeSettings, VibeEncodingRecord, VibeExportDocument,
    VibeExportFormat, VibeId, VibeSourceIdentity,
};

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

#[derive(Clone, Debug, PartialEq)]
pub struct EnsureVibeEncoding {
    pub vibe_id: VibeId,
    pub source: VibeSourceIdentity,
    pub image: String,
    pub settings: VibeEncodeSettings,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnsuredVibeEncoding {
    pub record: VibeEncodingRecord,
    pub created: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportVibeDocument {
    pub file_name: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportEmbeddedPngVibeDocument {
    pub file_name: String,
    pub png_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportedVibeDocuments {
    pub entries: Vec<VibeDocumentEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportVibeDocument {
    pub vibe_ids: Vec<VibeId>,
    pub format: VibeExportFormat,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedVibeDocument {
    pub document: VibeExportDocument,
}
