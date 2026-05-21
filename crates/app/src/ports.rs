use std::sync::{Arc, Mutex as StdMutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use atelier_adapter_database::{
    DatabaseArtifactRepository, DatabaseGalleryIndex, DatabaseGenerationPayloadStore,
    DatabasePromptResourceRepository, DatabaseResourceCatalogRepository, DatabaseVibeRepository,
};
use atelier_adapter_image_codec::{
    ImageCodec, ImageCodecVariantBuilder, ImageMetadataBlobStore, ImageSourceReader,
    ImageVariantSettingsProvider,
};
use atelier_adapter_novelai::{NovelAiClientFactory, ResolverBackedNovelAiAdapter};
use atelier_adapter_storage_fs::{FileSystemResourceBlobStore, FileSystemResourceContentReader};
use atelier_artifacts::{
    ArtifactKind, ArtifactRecord, ArtifactResult, ArtifactService, RegisterArtifactRequest,
    VisualAssetRef, VisualAssetRole,
};
use atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use atelier_gallery::{GalleryItem, GalleryResult, GalleryService};
use atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageStreamResult, NovelAiGenerationClient,
};
use atelier_kernel::{
    GenerationPayloadStore, KernelClock, KernelDirectorPorts, KernelEvent, KernelEventSink,
    KernelGenerationPorts, KernelPreciseReferencePorts, KernelResult, KernelVibePorts,
    PreparedGenerationPayload, SubmittedGenerationPayload,
};
use atelier_precise_reference::{
    PreciseReferenceError, PreciseReferenceImage, PreciseReferenceResult,
};
use atelier_prompt_resources::{
    CompilePromptRequest, CompiledPrompt, PromptCompiler, PromptResourceResult,
};
use atelier_resource_catalog::{
    BlobWriteIntent, BuiltResourceVariant, CreateVariantRequest, RegisterResourceRequest,
    ResourceCatalog, ResourceRef, ResourceResult, ResourceVariantKind, VariantId,
};
use atelier_safety::{SafetyAssessment, SafetyError, SafetyResult, SafetyScanInput, SafetyScanner};
use atelier_secrets::{ApiKeyRegistryService, SecretStore};
use atelier_settings::{ImageVariantSettings, WorkspaceSettings};
use atelier_vibe::{
    EmbeddedVibeDocumentExtractor, EncodeVibeRequest, EncodedVibe, NovelAiVibeClient,
    VibeDocumentEntry, VibeDomainResult, VibeEncodeSettings, VibeEncodingRecord, VibeError,
    VibeErrorKind, VibeId, VibeRepository, VibeResult, VibeSourceIdentity,
};

use crate::events::AppEventHub;

pub type AppApiKeyService<S, F> = ApiKeyRegistryService<
    atelier_adapter_database::DatabaseApiKeyRegistryStore,
    S,
    atelier_adapter_novelai::NovelAiSubscriptionProbeClient<F>,
>;

pub type AppNovelAiAdapter<S, F> = ResolverBackedNovelAiAdapter<AppApiKeyService<S, F>, F>;
pub type AppBlobStore = ImageMetadataBlobStore<FileSystemResourceBlobStore>;
pub type AppVariantBuilder =
    ImageCodecVariantBuilder<AppImageSourceReader, SharedWorkspaceSettings>;
pub type AppResourceCatalog =
    ResourceCatalog<DatabaseResourceCatalogRepository, AppBlobStore, AppVariantBuilder>;
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
    pub settings_state: SharedWorkspaceSettings,
    pub safety_scanner: Option<Arc<dyn SafetyScanner>>,
}

#[derive(Clone, Debug)]
pub struct SharedWorkspaceSettings {
    inner: Arc<StdMutex<WorkspaceSettings>>,
}

impl SharedWorkspaceSettings {
    #[must_use]
    pub fn new(settings: WorkspaceSettings) -> Self {
        Self {
            inner: Arc::new(StdMutex::new(settings)),
        }
    }

    pub fn replace(&self, settings: WorkspaceSettings) {
        if let Ok(mut current) = self.inner.lock() {
            *current = settings;
        }
    }
}

impl ImageVariantSettingsProvider for SharedWorkspaceSettings {
    fn image_variant_settings(&self) -> ImageVariantSettings {
        self.inner
            .lock()
            .map(|settings| settings.image_variants)
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub struct AppImageSourceReader {
    reader: AppResourceReader,
}

impl AppImageSourceReader {
    #[must_use]
    pub const fn new(reader: AppResourceReader) -> Self {
        Self { reader }
    }
}

#[async_trait]
impl ImageSourceReader for AppImageSourceReader {
    async fn read_image_source_bytes(&self, source: &ResourceRef) -> ResourceResult<Vec<u8>> {
        self.reader
            .read_resource_bytes(source)
            .await
            .map(|content| content.bytes)
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
        payload_ref: &atelier_jobs::JobPayloadRef,
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
        mut request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord> {
        self.add_generated_gallery_variants(&mut request).await;
        self.artifacts.register_artifact(request).await
    }

    async fn score_image(&self, resource: ResourceRef) -> SafetyResult<Option<SafetyAssessment>> {
        self.score_with_scanner(resource).await
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
impl<S, F, E> NovelAiDirectorClient for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        self.novelai.run_director_tool(request).await
    }
}

#[async_trait]
impl<S, F, E> KernelDirectorPorts for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    async fn register_director_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        self.resources.register_resource(request).await
    }

    async fn register_director_artifact(
        &self,
        mut request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord> {
        self.add_generated_gallery_variants(&mut request).await;
        self.artifacts.register_artifact(request).await
    }

    async fn score_director_image(
        &self,
        resource: ResourceRef,
    ) -> SafetyResult<Option<SafetyAssessment>> {
        self.score_with_scanner(resource).await
    }

    async fn index_director_gallery_item(
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

impl<S, F, E> AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    async fn add_generated_gallery_variants(&self, request: &mut RegisterArtifactRequest) {
        if !matches!(
            request.kind,
            ArtifactKind::GeneratedImage | ArtifactKind::DirectorResult
        ) {
            return;
        }
        let settings = self.settings_state.image_variant_settings();
        let Ok(content) = self
            .resource_reader
            .read_resource_bytes(&request.primary_resource)
            .await
        else {
            return;
        };
        let Ok(source) = ImageCodec::decode_source(&content.bytes) else {
            return;
        };

        for (kind, role) in [
            (ResourceVariantKind::Thumbnail, VisualAssetRole::Thumbnail),
            (ResourceVariantKind::Preview, VisualAssetRole::Preview),
            (ResourceVariantKind::Sanitized, VisualAssetRole::Sanitized),
            (ResourceVariantKind::Export, VisualAssetRole::Export),
        ] {
            if request
                .assets
                .iter()
                .any(|asset| asset.variant_kind == Some(kind))
            {
                continue;
            }
            let Ok(encoded) = source.build_variant(kind, settings) else {
                continue;
            };
            let variant_id = VariantId::new(format!(
                "variant:{}:{}",
                request.id.as_str(),
                resource_variant_kind_as_str(kind)
            ));
            let variant = match self
                .resources
                .create_built_variant(
                    CreateVariantRequest {
                        source: request.primary_resource.clone(),
                        variant_id,
                        kind,
                    },
                    BuiltResourceVariant {
                        blob: BlobWriteIntent::Bytes(encoded.bytes),
                    },
                )
                .await
            {
                Ok(variant) => variant,
                Err(_error) => continue,
            };
            request.assets.push(VisualAssetRef {
                role,
                resource: ResourceRef::new(request.primary_resource.id.clone(), Some(variant.id)),
                variant_kind: Some(kind),
            });
        }
    }

    async fn score_with_scanner(
        &self,
        resource: ResourceRef,
    ) -> SafetyResult<Option<SafetyAssessment>> {
        let Some(scanner) = &self.safety_scanner else {
            return Ok(None);
        };
        let content = self
            .resource_reader
            .read_resource_bytes(&resource)
            .await
            .map_err(|error| SafetyError::scanner(error.to_string()))?;
        scanner
            .scan_image(SafetyScanInput {
                resource,
                bytes: content.bytes,
                mime_type: None,
            })
            .await
            .map(Some)
    }
}

const fn resource_variant_kind_as_str(value: ResourceVariantKind) -> &'static str {
    match value {
        ResourceVariantKind::Original => "original",
        ResourceVariantKind::Preview => "preview",
        ResourceVariantKind::Thumbnail => "thumbnail",
        ResourceVariantKind::Sanitized => "sanitized",
        ResourceVariantKind::Export => "export",
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
