use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use nai_atelier_adapter_database::{
    DatabaseArtifactRepository, DatabaseGalleryIndex, DatabaseGenerationPayloadStore,
    DatabasePromptResourceRepository, DatabaseResourceCatalogRepository, DatabaseVibeRepository,
};
use nai_atelier_adapter_novelai::{NovelAiClientFactory, ResolverBackedNovelAiAdapter};
use nai_atelier_adapter_storage_fs::{
    FileSystemResourceBlobStore, FileSystemResourceContentReader,
};
use nai_atelier_artifacts::{
    ArtifactRecord, ArtifactResult, ArtifactService, RegisterArtifactRequest,
};
use nai_atelier_gallery::{GalleryItem, GalleryResult, GalleryService};
use nai_atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageStreamResult, NovelAiGenerationClient,
};
use nai_atelier_kernel::{
    GenerationPayloadStore, KernelClock, KernelEvent, KernelEventSink, KernelGenerationPorts,
    KernelPreciseReferencePorts, KernelResult, KernelVibePorts, PreparedGenerationPayload,
    SubmittedGenerationPayload,
};
use nai_atelier_precise_reference::{
    PreciseReferenceError, PreciseReferenceImage, PreciseReferenceResult,
};
use nai_atelier_prompt_resources::{
    CompilePromptRequest, CompiledPrompt, PromptCompiler, PromptResourceResult,
};
use nai_atelier_resource_catalog::{
    BuildVariantRequest, BuiltResourceVariant, RegisterResourceRequest, ResourceCatalog,
    ResourceCatalogError, ResourceRef, ResourceResult, ResourceVariantBuilder,
};
use nai_atelier_safety::{SafetyAssessment, SafetyResult};
use nai_atelier_secrets::{ApiKeyRegistryService, SecretStore};
use nai_atelier_vibe::{
    EmbeddedVibeDocumentExtractor, EncodeVibeRequest, EncodedVibe, NovelAiVibeClient,
    VibeDocumentEntry, VibeDomainResult, VibeEncodeSettings, VibeEncodingRecord, VibeError,
    VibeErrorKind, VibeId, VibeRepository, VibeResult, VibeSourceIdentity,
};

use crate::events::AppEventHub;

pub type AppApiKeyService<S, F> = ApiKeyRegistryService<
    nai_atelier_adapter_database::DatabaseApiKeyRegistryStore,
    S,
    nai_atelier_adapter_novelai::NovelAiSubscriptionProbeClient<F>,
>;

pub type AppNovelAiAdapter<S, F> = ResolverBackedNovelAiAdapter<AppApiKeyService<S, F>, F>;
pub type AppResourceCatalog = ResourceCatalog<
    DatabaseResourceCatalogRepository,
    FileSystemResourceBlobStore,
    NoopVariantBuilder,
>;
pub type AppArtifactService =
    ArtifactService<DatabaseArtifactRepository, DatabaseResourceCatalogRepository>;
pub type AppGalleryService = GalleryService<DatabaseGalleryIndex>;
pub type AppResourceReader = FileSystemResourceContentReader<DatabaseResourceCatalogRepository>;

pub struct AppKernelPorts<S, F, E> {
    pub payloads: DatabaseGenerationPayloadStore,
    pub prompt_compiler: PromptCompiler<DatabasePromptResourceRepository>,
    pub novelai: AppNovelAiAdapter<S, F>,
    pub resources: AppResourceCatalog,
    pub artifacts: AppArtifactService,
    pub gallery: AppGalleryService,
    pub resource_reader: AppResourceReader,
    pub vibes: DatabaseVibeRepository,
    pub extractor: E,
    pub events: AppEventHub,
}

#[derive(Clone, Debug)]
pub struct NoopVariantBuilder;

#[async_trait]
impl ResourceVariantBuilder for NoopVariantBuilder {
    async fn build_variant(
        &self,
        _request: BuildVariantRequest,
    ) -> ResourceResult<BuiltResourceVariant> {
        Err(ResourceCatalogError::variant_builder(
            "app does not build resource variants yet",
        ))
    }
}

#[async_trait]
impl<S, F, E> GenerationPayloadStore for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    async fn save_submitted_payload(
        &self,
        payload: SubmittedGenerationPayload,
    ) -> KernelResult<()> {
        self.payloads.save_submitted_payload(payload).await
    }

    async fn get_submitted_payload(
        &self,
        payload_ref: &nai_atelier_jobs::JobPayloadRef,
    ) -> KernelResult<Option<SubmittedGenerationPayload>> {
        self.payloads.get_submitted_payload(payload_ref).await
    }

    async fn save_prepared_payload(&self, payload: PreparedGenerationPayload) -> KernelResult<()> {
        self.payloads.save_prepared_payload(payload).await
    }
}

impl<S, F, E> KernelClock for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    fn now_ms(&self) -> u64 {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        u64::try_from(millis).unwrap_or(u64::MAX)
    }
}

#[async_trait]
impl<S, F, E> KernelEventSink for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    async fn emit(&self, event: KernelEvent) {
        self.events.push_kernel_event(event);
    }
}

#[async_trait]
impl<S, F, E> NovelAiGenerationClient for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        self.novelai.generate(request).await
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        self.novelai.generate_stream(request).await
    }
}

#[async_trait]
impl<S, F, E> KernelGenerationPorts for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    async fn compile_prompt(
        &self,
        request: CompilePromptRequest,
    ) -> PromptResourceResult<CompiledPrompt> {
        self.prompt_compiler.compile(request).await
    }

    async fn register_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        self.resources.register_resource(request).await
    }

    async fn register_artifact(
        &self,
        request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord> {
        self.artifacts.register_artifact(request).await
    }

    async fn score_image(&self, _resource: ResourceRef) -> SafetyResult<Option<SafetyAssessment>> {
        Ok(None)
    }

    async fn index_gallery_item(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety_assessment: Option<SafetyAssessment>,
    ) -> GalleryResult<GalleryItem> {
        self.gallery
            .index_artifact(artifact, indexed_at_ms, safety_assessment)
            .await
    }
}

#[async_trait]
impl<S, F, E> NovelAiVibeClient for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        self.novelai.encode_vibe(request).await
    }
}

#[async_trait]
impl<S, F, E> EmbeddedVibeDocumentExtractor for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    async fn extract_embedded_vibe_document_from_png(
        &self,
        png_bytes: &[u8],
    ) -> VibeDomainResult<Option<String>> {
        self.extractor
            .extract_embedded_vibe_document_from_png(png_bytes)
            .await
    }
}

#[async_trait]
impl<S, F, E> VibeRepository for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    async fn insert_document(&self, entry: VibeDocumentEntry) -> VibeDomainResult<VibeId> {
        self.vibes.insert_document(entry).await
    }

    async fn get_document(&self, id: &VibeId) -> VibeDomainResult<Option<VibeDocumentEntry>> {
        self.vibes.get_document(id).await
    }

    async fn find_cached_encoding(
        &self,
        source: &VibeSourceIdentity,
        settings: &VibeEncodeSettings,
    ) -> VibeDomainResult<Option<VibeEncodingRecord>> {
        self.vibes.find_cached_encoding(source, settings).await
    }

    async fn save_encoding(&self, record: VibeEncodingRecord) -> VibeDomainResult<()> {
        self.vibes.save_encoding(record).await
    }
}

#[async_trait]
impl<S, F, E> KernelVibePorts for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    async fn register_vibe_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        self.resources.register_resource(request).await
    }

    async fn read_vibe_document_resource(
        &self,
        reference: &ResourceRef,
    ) -> VibeDomainResult<String> {
        self.resource_reader
            .read_resource_text(reference)
            .await
            .map_err(|error| VibeError::new(VibeErrorKind::Repository, error.to_string()))
    }
}

#[async_trait]
impl<S, F, E> KernelPreciseReferencePorts for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    async fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage> {
        let content = self
            .resource_reader
            .read_resource_base64(source)
            .await
            .map_err(|error| PreciseReferenceError::not_found(error.to_string()))?;
        let kind = self
            .resource_reader
            .read_resource_bytes(source)
            .await
            .map_err(|error| PreciseReferenceError::not_found(error.to_string()))?
            .kind;
        Ok(PreciseReferenceImage {
            kind,
            payload: content,
        })
    }
}
