use atelier_artifacts::ArtifactId;
use atelier_director::RunDirectorToolRequest;
use atelier_gallery::GalleryItem;
use atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GenerationPlanContext, GenerationRequestPlan,
};
use atelier_jobs::{BatchId, JobId, JobPayloadRef};
use atelier_prompt_resources::CompiledPrompt;
use atelier_resource_catalog::ResourceRef;
use atelier_vibe::{
    VibeDocumentEntry, VibeEncodeSettings, VibeEncodingRecord, VibeExportDocument,
    VibeExportFormat, VibeId, VibeSourceIdentity,
};

#[derive(Clone, Debug, PartialEq)]
pub enum GenerationWorkRequest {
    Image(GenerateImageRequest),
    Stream(GenerateImageStreamRequest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledGenerationCharacterPrompts {
    pub prompt: Option<CompiledPrompt>,
    pub negative_prompt: Option<CompiledPrompt>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledGenerationPrompts {
    pub prompt: CompiledPrompt,
    pub negative_prompt: Option<CompiledPrompt>,
    pub characters: Vec<CompiledGenerationCharacterPrompts>,
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

    #[must_use]
    pub fn negative_prompt(&self) -> Option<&str> {
        match self {
            Self::Image(request) => request.negative_prompt.as_deref(),
            Self::Stream(request) => request.base.negative_prompt.as_deref(),
        }
    }

    #[must_use]
    pub fn characters(&self) -> Option<&[atelier_generation::Character]> {
        match self {
            Self::Image(request) => request.characters.as_deref(),
            Self::Stream(request) => request.base.characters.as_deref(),
        }
    }

    #[must_use]
    pub fn with_compiled_prompts(mut self, compiled: &CompiledGenerationPrompts) -> Self {
        match &mut self {
            Self::Image(request) => apply_compiled_prompts(request, compiled),
            Self::Stream(request) => apply_compiled_prompts(&mut request.base, compiled),
        }
        self
    }
}

fn apply_compiled_prompts(
    request: &mut GenerateImageRequest,
    compiled: &CompiledGenerationPrompts,
) {
    request.prompt.clone_from(&compiled.prompt.expanded_prompt);
    if let Some(negative_prompt) = &compiled.negative_prompt {
        request.negative_prompt = Some(negative_prompt.expanded_prompt.clone());
    }
    if let Some(characters) = &mut request.characters {
        for (character, compiled_character) in characters.iter_mut().zip(&compiled.characters) {
            if let Some(prompt) = &compiled_character.prompt {
                character.prompt.clone_from(&prompt.expanded_prompt);
            }
            if let Some(negative_prompt) = &compiled_character.negative_prompt {
                character.negative_prompt = Some(negative_prompt.expanded_prompt.clone());
            }
        }
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
pub struct SubmitGenerationBatchJob {
    pub job_id: JobId,
    pub request: GenerationWorkRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubmitGenerationBatch {
    pub batch_id: BatchId,
    pub jobs: Vec<SubmitGenerationBatchJob>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDirectorTool {
    pub run_id: String,
    pub request: RunDirectorToolRequest,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RanDirectorTool {
    pub resource: ResourceRef,
    pub artifact_id: ArtifactId,
    pub item: GalleryItem,
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
