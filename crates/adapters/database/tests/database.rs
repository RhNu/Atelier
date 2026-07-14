use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use atelier_adapter_database::{
    DatabaseApiKeyRegistryStore, DatabaseArtifactRepository, DatabaseConnection,
    DatabaseGalleryIndex, DatabaseGenerationPayloadStore, DatabaseResourceCatalogRepository,
    DatabaseVibeRepository, GalleryHardDeletePlan, GalleryTransientOwner,
};
use atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest,
    ArtifactRepository, ArtifactResult, ArtifactService, ArtifactSource, RegisterArtifactRequest,
    VisualAssetRef, VisualAssetRole,
};
use atelier_gallery::{
    GalleryIndex, GalleryItem, GalleryItemId, GalleryQuery, GalleryResult, GallerySafetyOverride,
    GalleryService, GallerySourceKind,
};
use atelier_generation::{
    GenerateImageRequest, GeneratedImage, GenerationPlanContext, GenerationResult, ImageModel,
    ImageSize, ImageStreamResult, NovelAiGenerationClient, plan_generation_request,
};
use atelier_jobs::{BatchId, JobId, JobPayloadRef};
use atelier_kernel::{
    GenerationPayloadStore, GenerationWorkRequest, KernelClock, KernelEvent, KernelEventSink,
    KernelGenerationPorts, KernelRuntime, PreparedGenerationPayload, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
use atelier_prompt_resources::{
    CompilePromptRequest, CompiledPrompt, PromptResourceResult, PromptTrace,
};
use atelier_resource_catalog::{
    BlobId, BlobWriteIntent, BuildVariantRequest, BuiltResourceVariant, CreateVariantRequest,
    RegisterResourceRequest, ResourceBlobStore, ResourceCatalog, ResourceCatalogError,
    ResourceCatalogRepository, ResourceId, ResourceKind, ResourceLifecycle, ResourceMetadata,
    ResourceOwner, ResourceOwnerKind, ResourceRef, ResourceRelation, ResourceResult, ResourceState,
    ResourceVariantBuilder, ResourceVariantKind, StagedBlob, StagedBlobToken, VariantId,
};
use atelier_safety::{ImageSafetyScore, SafetyAssessment, SafetyLabel, SafetyResult};
use atelier_secrets::{
    ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, SecretRecordId, SecretsErrorKind,
};
use atelier_vibe::{
    VibeDocumentEntry, VibeDocumentResources, VibeDocumentSummary, VibeEncodeSettings,
    VibeEncodingRecord, VibeId, VibeModel, VibeRepository, VibeSourceIdentity,
};
use futures_executor::block_on;
use rusqlite::Connection;

#[path = "database/gallery_and_workflow.rs"]
mod gallery_and_workflow;
#[path = "database/migrations_and_keys.rs"]
mod migrations_and_keys;
#[path = "database/resources_and_payloads.rs"]
mod resources_and_payloads;

fn generated_resource(id: &str, bytes: Vec<u8>) -> RegisterResourceRequest {
    RegisterResourceRequest {
        resource_id: ResourceId::new(id),
        kind: ResourceKind::GeneratedImage,
        lifecycle: ResourceLifecycle::JobScoped,
        owner: ResourceOwner::new(ResourceOwnerKind::Job, "job-1"),
        relation: ResourceRelation::Primary,
        blob: BlobWriteIntent::Bytes(bytes),
    }
}

fn artifact_record(id: &str, seed: i64, source: ArtifactSource) -> ArtifactRecord {
    let artifact_id = ArtifactId::new(id);
    let resource = ResourceRef::base(ResourceId::new(format!("resource-{id}")));
    ArtifactRecord {
        id: artifact_id,
        kind: ArtifactKind::GeneratedImage,
        source,
        primary_resource: resource.clone(),
        metadata: ArtifactMetadata {
            seed: Some(seed),
            sample_index: Some(0),
            model_name: Some("nai-diffusion-4-5-full".to_owned()),
            extensions: BTreeMap::from([("prompt".to_owned(), "1girl".to_owned())]),
        },
        replay: Some(ArtifactReplayManifest {
            payload_ref: Some(format!("generation-submitted:{id}")),
            prepared_payload_ref: Some(format!("generation-prepared:{id}")),
            prompt_snapshot: Some("1girl".to_owned()),
            negative_prompt_snapshot: Some("lowres".to_owned()),
        }),
        assets: vec![VisualAssetRef {
            role: VisualAssetRole::Original,
            resource,
            variant_kind: Some(ResourceVariantKind::Original),
        }],
    }
}

fn api_key_record(id: &str, display_name: &str, is_active: bool) -> ApiKeyRecord {
    ApiKeyRecord {
        id: ApiKeyId::new(id),
        display_name: display_name.to_owned(),
        secret_record_id: SecretRecordId::for_api_key(&ApiKeyId::new(id)),
        is_active,
    }
}

fn artifact_request(id: &str, primary_resource: ResourceRef) -> RegisterArtifactRequest {
    RegisterArtifactRequest {
        id: ArtifactId::new(id),
        kind: ArtifactKind::GeneratedImage,
        source: ArtifactSource::GenerationJob {
            job_id: format!("job-{id}"),
            batch_id: None,
        },
        primary_resource,
        metadata: ArtifactMetadata::default(),
        replay: None,
        assets: Vec::new(),
    }
}

#[derive(Clone, Default)]
struct MemoryBlobStore {
    state: Arc<Mutex<MemoryBlobState>>,
}

#[derive(Default)]
struct MemoryBlobState {
    next: u32,
}

#[async_trait]
impl ResourceBlobStore for MemoryBlobStore {
    async fn stage_blob(
        &self,
        intent: BlobWriteIntent,
    ) -> Result<StagedBlob, ResourceCatalogError> {
        let BlobWriteIntent::Bytes(bytes) = intent;
        let next = {
            let mut state = self.state.lock().unwrap();
            state.next += 1;
            state.next
        };
        let token = StagedBlobToken::new(format!("staged-{next}"));
        let blob_id = BlobId::new(format!("blob-{next}"));
        Ok(StagedBlob {
            token,
            blob_id,
            metadata: ResourceMetadata {
                byte_size: Some(bytes.len() as u64),
                ..ResourceMetadata::default()
            },
        })
    }

    async fn finalize_blob(&self, _staged: &StagedBlobToken) -> Result<(), ResourceCatalogError> {
        Ok(())
    }

    async fn abort_staged_blob(
        &self,
        _staged: &StagedBlobToken,
    ) -> Result<(), ResourceCatalogError> {
        Ok(())
    }

    async fn delete_blob(&self, _blob_id: &BlobId) -> Result<(), ResourceCatalogError> {
        Ok(())
    }

    async fn blob_exists(&self, _blob_id: &BlobId) -> Result<bool, ResourceCatalogError> {
        Ok(true)
    }
}

#[derive(Clone)]
struct NullVariantBuilder;

#[async_trait]
impl ResourceVariantBuilder for NullVariantBuilder {
    async fn build_variant(
        &self,
        _request: BuildVariantRequest,
    ) -> Result<BuiltResourceVariant, ResourceCatalogError> {
        Ok(BuiltResourceVariant {
            blob: BlobWriteIntent::Bytes(vec![7; 7]),
        })
    }
}

#[derive(Clone)]
struct DatabaseWorkflowPorts {
    payloads: DatabaseGenerationPayloadStore,
    catalog:
        ResourceCatalog<DatabaseResourceCatalogRepository, MemoryBlobStore, NullVariantBuilder>,
    artifacts: ArtifactService<DatabaseArtifactRepository, DatabaseResourceCatalogRepository>,
    gallery: GalleryService<DatabaseGalleryIndex>,
    generated: Arc<Mutex<u32>>,
}

impl DatabaseWorkflowPorts {
    fn new(connection: DatabaseConnection) -> Self {
        let resource_repository = DatabaseResourceCatalogRepository::new(connection.clone());
        Self {
            payloads: DatabaseGenerationPayloadStore::new(connection.clone()),
            catalog: ResourceCatalog::new(
                resource_repository.clone(),
                MemoryBlobStore::default(),
                NullVariantBuilder,
            ),
            artifacts: ArtifactService::new(
                DatabaseArtifactRepository::new(connection.clone()),
                resource_repository,
            ),
            gallery: GalleryService::new(DatabaseGalleryIndex::new(connection)),
            generated: Arc::default(),
        }
    }

    fn generate_call_count(&self) -> u32 {
        *self.generated.lock().unwrap()
    }
}

#[async_trait]
impl GenerationPayloadStore for DatabaseWorkflowPorts {
    async fn save_submitted_payload(
        &self,
        payload: SubmittedGenerationPayload,
    ) -> atelier_kernel::KernelResult<()> {
        self.payloads.save_submitted_payload(payload).await
    }

    async fn get_submitted_payload(
        &self,
        payload_ref: &JobPayloadRef,
    ) -> atelier_kernel::KernelResult<Option<SubmittedGenerationPayload>> {
        self.payloads.get_submitted_payload(payload_ref).await
    }

    async fn save_prepared_payload(
        &self,
        payload: PreparedGenerationPayload,
    ) -> atelier_kernel::KernelResult<()> {
        self.payloads.save_prepared_payload(payload).await
    }
}

impl KernelClock for DatabaseWorkflowPorts {
    fn now_ms(&self) -> u64 {
        123
    }
}

#[async_trait]
impl KernelEventSink for DatabaseWorkflowPorts {
    async fn emit(&self, _event: KernelEvent) {}
}

#[async_trait]
impl NovelAiGenerationClient for DatabaseWorkflowPorts {
    async fn generate(
        &self,
        _request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        *self.generated.lock().unwrap() += 1;
        Ok(vec![GeneratedImage {
            bytes: vec![1, 2, 3],
            mime_type: Some("image/png".to_owned()),
            seed: Some(99),
        }])
    }

    async fn generate_stream(
        &self,
        _request: atelier_generation::GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        Ok(Box::pin(futures_util::stream::empty()))
    }
}

#[async_trait]
impl KernelGenerationPorts for DatabaseWorkflowPorts {
    async fn compile_prompt(
        &self,
        request: CompilePromptRequest,
    ) -> PromptResourceResult<CompiledPrompt> {
        Ok(CompiledPrompt {
            expanded_prompt: request.prompt.clone(),
            trace: PromptTrace {
                raw_prompt: request.prompt.clone(),
                expanded_prompt: request.prompt,
                function_calls: Vec::new(),
            },
        })
    }

    async fn register_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        self.catalog.register_resource(request).await
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
