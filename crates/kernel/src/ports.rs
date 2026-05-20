use async_trait::async_trait;
use nai_atelier_artifacts::{ArtifactRecord, ArtifactResult, RegisterArtifactRequest};
use nai_atelier_director::NovelAiDirectorClient;
use nai_atelier_gallery::{GalleryItem, GalleryResult};
use nai_atelier_generation::NovelAiGenerationClient;
use nai_atelier_jobs::JobPayloadRef;
use nai_atelier_precise_reference::{PreciseReferenceImage, PreciseReferenceResult};
use nai_atelier_prompt_resources::{CompilePromptRequest, CompiledPrompt, PromptResourceResult};
use nai_atelier_resource_catalog::{RegisterResourceRequest, ResourceRef, ResourceResult};
use nai_atelier_safety::{SafetyAssessment, SafetyResult};
use nai_atelier_vibe::{EmbeddedVibeDocumentExtractor, NovelAiVibeClient, VibeRepository};

use crate::{KernelEvent, KernelResult, PreparedGenerationPayload, SubmittedGenerationPayload};

#[async_trait]
pub trait KernelEventSink: Send + Sync {
    async fn emit(&self, event: KernelEvent);
}

pub trait KernelClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

#[async_trait]
pub trait GenerationPayloadStore: Send + Sync {
    async fn save_submitted_payload(&self, payload: SubmittedGenerationPayload)
    -> KernelResult<()>;

    async fn get_submitted_payload(
        &self,
        payload_ref: &JobPayloadRef,
    ) -> KernelResult<Option<SubmittedGenerationPayload>>;

    async fn save_prepared_payload(&self, payload: PreparedGenerationPayload) -> KernelResult<()>;
}

#[async_trait]
pub trait KernelGenerationPorts: NovelAiGenerationClient + Send + Sync {
    async fn compile_prompt(
        &self,
        request: CompilePromptRequest,
    ) -> PromptResourceResult<CompiledPrompt>;

    async fn register_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef>;

    async fn register_artifact(
        &self,
        request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord>;

    async fn score_image(&self, resource: ResourceRef) -> SafetyResult<Option<SafetyAssessment>>;

    async fn index_gallery_item(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety_assessment: Option<SafetyAssessment>,
    ) -> GalleryResult<GalleryItem>;
}

#[async_trait]
pub trait KernelDirectorPorts: NovelAiDirectorClient + Send + Sync {
    async fn register_director_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef>;

    async fn register_director_artifact(
        &self,
        request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord>;

    async fn score_director_image(
        &self,
        resource: ResourceRef,
    ) -> SafetyResult<Option<SafetyAssessment>>;

    async fn index_director_gallery_item(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety_assessment: Option<SafetyAssessment>,
    ) -> GalleryResult<GalleryItem>;
}

#[async_trait]
pub trait KernelVibePorts:
    NovelAiVibeClient + EmbeddedVibeDocumentExtractor + VibeRepository + Send + Sync
{
    async fn register_vibe_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef>;

    async fn read_vibe_document_resource(
        &self,
        reference: &ResourceRef,
    ) -> nai_atelier_vibe::VibeDomainResult<String>;
}

#[async_trait]
pub trait KernelPreciseReferencePorts: Send + Sync {
    async fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage>;
}
