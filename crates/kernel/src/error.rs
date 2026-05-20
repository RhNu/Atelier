use nai_atelier_artifacts::ArtifactError;
use nai_atelier_foundation::NovelAiError;
use nai_atelier_gallery::GalleryError;
use nai_atelier_generation::GenerationError;
use nai_atelier_jobs::{JobPayloadRef, JobQueueError};
use nai_atelier_precise_reference::PreciseReferenceError;
use nai_atelier_prompt_resources::PromptResourceError;
use nai_atelier_resource_catalog::ResourceCatalogError;
use nai_atelier_safety::SafetyError;
use nai_atelier_vibe::VibeError;
use thiserror::Error;

pub type KernelResult<T> = Result<T, KernelError>;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum KernelError {
    #[error("prompt resource workflow failed: {0}")]
    PromptResource(#[from] PromptResourceError),
    #[error("generation planning failed: {0}")]
    Generation(#[from] GenerationError),
    #[error("job queue failed: {0}")]
    JobQueue(#[from] JobQueueError),
    #[error("resource catalog failed: {0}")]
    ResourceCatalog(#[from] ResourceCatalogError),
    #[error("artifact registration failed: {0}")]
    Artifact(#[from] ArtifactError),
    #[error("gallery indexing failed: {0}")]
    Gallery(#[from] GalleryError),
    #[error("safety scan failed: {0}")]
    Safety(#[from] SafetyError),
    #[error("vibe workflow failed: {0}")]
    Vibe(#[from] VibeError),
    #[error("precise reference workflow failed: {0}")]
    PreciseReference(#[from] PreciseReferenceError),
    #[error("novelai generation failed: {0}")]
    NovelAi(#[from] NovelAiError),
    #[error("payload store failed: {0}")]
    PayloadStore(String),
    #[error("submitted payload `{0:?}` does not exist")]
    MissingSubmittedPayload(JobPayloadRef),
    #[error("stream sample {sample_index} did not contain valid image data: {message}")]
    InvalidStreamImage { sample_index: u32, message: String },
    #[error("generation completed without a persistable image")]
    MissingGeneratedImage,
}
