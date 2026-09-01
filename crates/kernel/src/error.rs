use atelier_artifacts::ArtifactError;
use atelier_director::DirectorClientError;
use atelier_gallery::GalleryError;
use atelier_generation::{GenerationClientError, GenerationError};
use atelier_jobs::{JobPayloadRef, JobQueueError};
use atelier_precise_reference::PreciseReferenceError;
use atelier_prompt_resources::PromptResourceError;
use atelier_resource_catalog::ResourceCatalogError;
use atelier_safety::SafetyError;
use atelier_vibe::{VibeClientError, VibeError};
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
    #[error("generation client failed: {0}")]
    GenerationClient(#[from] GenerationClientError),
    #[error("director client failed: {0}")]
    DirectorClient(#[from] DirectorClientError),
    #[error("vibe client failed: {0}")]
    VibeClient(#[from] VibeClientError),
    #[error("payload store failed: {0}")]
    PayloadStore(String),
    #[error("submitted payload `{0:?}` does not exist")]
    MissingSubmittedPayload(JobPayloadRef),
    #[error("stream sample {sample_index} did not contain valid image data: {message}")]
    InvalidStreamImage { sample_index: u32, message: String },
    #[error("generation completed without a persistable image")]
    MissingGeneratedImage,
    #[error("generation stream was cancelled")]
    GenerationCancelled,
}
