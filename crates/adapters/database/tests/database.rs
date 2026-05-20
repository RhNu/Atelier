use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use futures_executor::block_on;
use nai_atelier_adapter_database::{
    DatabaseArtifactRepository, DatabaseConnection, DatabaseGalleryIndex,
    DatabaseGenerationPayloadStore, DatabaseResourceCatalogRepository, DatabaseVibeRepository,
};
use nai_atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest,
    ArtifactRepository, ArtifactResult, ArtifactService, ArtifactSource, RegisterArtifactRequest,
    VisualAssetRef, VisualAssetRole,
};
use nai_atelier_gallery::{
    GalleryIndex, GalleryItem, GalleryItemId, GalleryQuery, GalleryResult, GallerySafetyOverride,
    GalleryService, GallerySourceKind,
};
use nai_atelier_generation::{
    GenerateImageRequest, GeneratedImage, GenerationPlanContext, GenerationResult, ImageModel,
    ImageSize, ImageStreamResult, NovelAiGenerationClient, plan_generation_request,
};
use nai_atelier_jobs::{BatchId, JobId, JobPayloadRef};
use nai_atelier_kernel::{
    GenerationPayloadStore, GenerationWorkRequest, KernelClock, KernelEvent, KernelEventSink,
    KernelGenerationPorts, KernelRuntime, PreparedGenerationPayload, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
use nai_atelier_prompt_resources::{
    CompilePromptRequest, CompiledPrompt, PromptResourceResult, PromptTrace,
};
use nai_atelier_resource_catalog::{
    BlobId, BlobWriteIntent, BuildVariantRequest, BuiltResourceVariant, CreateVariantRequest,
    RegisterResourceRequest, ResourceBlobStore, ResourceCatalog, ResourceCatalogError,
    ResourceCatalogRepository, ResourceId, ResourceKind, ResourceLifecycle, ResourceMetadata,
    ResourceOwner, ResourceOwnerKind, ResourceRef, ResourceRelation, ResourceResult, ResourceState,
    ResourceVariantBuilder, ResourceVariantKind, StagedBlob, StagedBlobToken, VariantId,
};
use nai_atelier_safety::{ImageSafetyScore, SafetyAssessment, SafetyResult};
use nai_atelier_vibe::{
    VibeDocumentEntry, VibeDocumentResources, VibeDocumentSummary, VibeEncodeSettings,
    VibeEncodingRecord, VibeId, VibeModel, VibeRepository, VibeSourceIdentity,
};

#[test]
fn migrations_are_idempotent_and_file_backed_database_reopens() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let first = DatabaseConnection::open(&path).unwrap();
        first.run_migrations().unwrap();
        let repository = DatabaseResourceCatalogRepository::new(first.clone());
        let catalog =
            ResourceCatalog::new(repository, MemoryBlobStore::default(), NullVariantBuilder);

        let reference = catalog
            .register_resource(generated_resource("persisted-res", vec![1, 2, 3]))
            .await
            .unwrap();
        drop(catalog);
        drop(first);

        let reopened = DatabaseConnection::open(&path).unwrap();
        reopened.run_migrations().unwrap();
        let repository = DatabaseResourceCatalogRepository::new(reopened);
        let record = repository.get_ready_record(&reference.id).await.unwrap();

        assert_eq!(record.unwrap().metadata.byte_size, Some(3));
    });
}

#[test]
fn resource_catalog_repository_round_trips_records_links_variants_and_orphans() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let repository = DatabaseResourceCatalogRepository::new(connection);
        let blob_store = MemoryBlobStore::default();
        let catalog = ResourceCatalog::new(repository.clone(), blob_store, NullVariantBuilder);
        let job_owner = ResourceOwner::new(ResourceOwnerKind::Job, "job-1");
        let gallery_owner = ResourceOwner::new(ResourceOwnerKind::GalleryItem, "gallery-1");

        let reference = catalog
            .register_resource(RegisterResourceRequest {
                owner: job_owner.clone(),
                ..generated_resource("res-1", vec![9])
            })
            .await
            .unwrap();
        catalog
            .attach_owner(
                &reference.id,
                gallery_owner.clone(),
                ResourceRelation::Reference,
            )
            .await
            .unwrap();
        let variant = catalog
            .create_variant(CreateVariantRequest {
                source: reference.clone(),
                variant_id: VariantId::new("preview-1"),
                kind: ResourceVariantKind::Preview,
            })
            .await
            .unwrap();
        catalog
            .detach_owner(&reference.id, &job_owner, ResourceRelation::Primary)
            .await
            .unwrap();
        repository
            .record_orphan_blob(&BlobId::new("sha256:orphan"))
            .await
            .unwrap();

        assert_eq!(
            catalog.list_by_owner(&gallery_owner).await.unwrap(),
            vec![reference.clone()]
        );
        assert_eq!(
            catalog.list_links_by_owner(&gallery_owner).await.unwrap()[0].relation,
            ResourceRelation::Reference
        );
        assert_eq!(catalog.get_variant(&variant.id).await.unwrap(), variant);
        assert_eq!(
            repository.scan_orphan_blobs().await.unwrap(),
            vec![BlobId::new("sha256:orphan")]
        );
        assert_eq!(
            repository
                .get_ready_record(&reference.id)
                .await
                .unwrap()
                .unwrap()
                .state,
            ResourceState::Ready
        );
    });
}

#[test]
fn resource_catalog_transactions_are_serialized_until_rollback() {
    block_on(async {
        let repository =
            DatabaseResourceCatalogRepository::new(DatabaseConnection::open_memory().unwrap());
        let tx = repository.begin_transaction().await.unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (acquired_tx, acquired_rx) = mpsc::channel();
        let repository_for_thread = repository.clone();

        let handle = thread::spawn(move || {
            started_tx.send(()).unwrap();
            let tx = block_on(repository_for_thread.begin_transaction()).unwrap();
            acquired_tx.send(()).unwrap();
            block_on(tx.rollback()).unwrap();
        });

        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(acquired_rx.recv_timeout(Duration::from_millis(50)).is_err());
        tx.rollback().await.unwrap();
        acquired_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        handle.join().unwrap();
    });
}

#[test]
fn generation_payload_store_keeps_submitted_payload_when_prepared_payload_shares_ref() {
    block_on(async {
        let store = DatabaseGenerationPayloadStore::new(DatabaseConnection::open_memory().unwrap());
        let request = GenerateImageRequest {
            prompt: "1girl".to_owned(),
            model: ImageModel::NaiDiffusion45Full,
            size: ImageSize::square(),
            seed: 42,
            ..Default::default()
        };
        let plan =
            plan_generation_request(request.clone(), GenerationPlanContext::default()).unwrap();
        let submitted = SubmittedGenerationPayload {
            payload_ref: JobPayloadRef::new("generation-submitted:job-1"),
            batch_id: BatchId::new("batch-1"),
            job_id: JobId::new("job-1"),
            request: GenerationWorkRequest::Image(request.clone()),
            context: GenerationPlanContext::default(),
        };
        let prepared = PreparedGenerationPayload {
            payload_ref: submitted.payload_ref.clone(),
            submitted_payload_ref: submitted.payload_ref.clone(),
            batch_id: submitted.batch_id.clone(),
            job_id: submitted.job_id.clone(),
            request: GenerationWorkRequest::Image(request),
            compiled_prompt: CompiledPrompt {
                expanded_prompt: "expanded prompt".to_owned(),
                trace: PromptTrace {
                    raw_prompt: "1girl".to_owned(),
                    expanded_prompt: "expanded prompt".to_owned(),
                    function_calls: Vec::new(),
                },
            },
            plan,
        };

        store
            .save_submitted_payload(submitted.clone())
            .await
            .unwrap();
        store.save_prepared_payload(prepared).await.unwrap();

        assert_eq!(
            store
                .get_submitted_payload(&submitted.payload_ref)
                .await
                .unwrap(),
            Some(submitted)
        );
        assert_eq!(
            store
                .get_submitted_payload(&JobPayloadRef::new("missing"))
                .await
                .unwrap(),
            None
        );
    });
}

#[test]
fn artifact_service_rejects_missing_or_mismatched_primary_variant() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let resource_repository = DatabaseResourceCatalogRepository::new(connection.clone());
        let catalog = ResourceCatalog::new(
            resource_repository.clone(),
            MemoryBlobStore::default(),
            NullVariantBuilder,
        );
        let artifacts = ArtifactService::new(
            DatabaseArtifactRepository::new(connection),
            resource_repository,
        );
        let first = catalog
            .register_resource(generated_resource("artifact-res-1", vec![1]))
            .await
            .unwrap();
        let second = catalog
            .register_resource(generated_resource("artifact-res-2", vec![2]))
            .await
            .unwrap();
        let second_variant = catalog
            .create_variant(CreateVariantRequest {
                source: second.clone(),
                variant_id: VariantId::new("artifact-preview-2"),
                kind: ResourceVariantKind::Preview,
            })
            .await
            .unwrap();

        let missing_error = artifacts
            .register_artifact(artifact_request(
                "artifact-missing-variant",
                ResourceRef::new(first.id.clone(), Some(VariantId::new("missing-variant"))),
            ))
            .await
            .unwrap_err();
        let mismatch_error = artifacts
            .register_artifact(artifact_request(
                "artifact-mismatched-variant",
                ResourceRef::new(first.id, Some(second_variant.id.clone())),
            ))
            .await
            .unwrap_err();
        let valid = artifacts
            .register_artifact(artifact_request(
                "artifact-valid-variant",
                ResourceRef::new(second.id, Some(second_variant.id)),
            ))
            .await
            .unwrap();

        assert!(missing_error.to_string().contains("variant does not exist"));
        assert!(
            mismatch_error
                .to_string()
                .contains("belongs to another resource")
        );
        assert_eq!(valid.id.as_str(), "artifact-valid-variant");
    });
}

#[test]
fn vibe_repository_round_trips_documents_and_cached_encodings() {
    block_on(async {
        let repository = DatabaseVibeRepository::new(DatabaseConnection::open_memory().unwrap());
        let settings = VibeEncodeSettings::new(VibeModel::NaiDiffusion45Full, 0.7).unwrap();
        let entry = VibeDocumentEntry {
            summary: VibeDocumentSummary {
                document_id: VibeId::new("vibe-1"),
                display_name: "Style A".to_owned(),
                has_image: true,
                available_model_keys: vec!["v4-5full".to_owned()],
                available_encoding_configs: vec![nai_atelier_vibe::VibeEncodingConfig {
                    model: VibeModel::NaiDiffusion45Full,
                    settings: settings.clone(),
                }],
            },
            resources: VibeDocumentResources {
                document: ResourceRef::base(ResourceId::new("vibe-document")),
                source_image: Some(ResourceRef::base(ResourceId::new("vibe-source"))),
                preview: Some(ResourceRef::base(ResourceId::new("vibe-preview"))),
                encodings: vec![ResourceRef::base(ResourceId::new("vibe-encoding"))],
            },
        };
        let source = VibeSourceIdentity::new_sha256("source-hash");
        let encoding = VibeEncodingRecord {
            vibe_id: VibeId::new("vibe-1"),
            source: source.clone(),
            settings: settings.clone(),
            resource: ResourceRef::base(ResourceId::new("cached-encoding")),
        };

        repository.insert_document(entry.clone()).await.unwrap();
        repository.save_encoding(encoding.clone()).await.unwrap();

        assert_eq!(
            repository
                .get_document(&VibeId::new("vibe-1"))
                .await
                .unwrap(),
            Some(entry)
        );
        assert_eq!(
            repository
                .find_cached_encoding(&source, &settings)
                .await
                .unwrap(),
            Some(encoding)
        );
    });
}

#[test]
fn artifact_and_gallery_persistence_supports_query_and_manual_override() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let artifacts = DatabaseArtifactRepository::new(connection.clone());
        let gallery = DatabaseGalleryIndex::new(connection);
        let first = artifact_record(
            "artifact-1",
            11,
            ArtifactSource::GenerationJob {
                job_id: "job-1".to_owned(),
                batch_id: Some("batch-1".to_owned()),
            },
        );
        let second = artifact_record(
            "artifact-2",
            22,
            ArtifactSource::DirectorRun {
                run_id: "director-1".to_owned(),
            },
        );
        let first_item = GalleryItem {
            id: GalleryItemId::from_artifact_id(&first.id),
            artifact_id: first.id.clone(),
            artifact_kind: first.kind,
            source: first.source.clone(),
            primary_resource: first.primary_resource.clone(),
            assets: first.assets.clone(),
            metadata: first.metadata.clone(),
            safety_assessment: Some(SafetyAssessment::new(
                first.primary_resource.clone(),
                ImageSafetyScore::new(0.25).unwrap(),
            )),
            manual_safety_override: None,
            indexed_at_ms: 100,
        };
        let second_item = GalleryItem {
            id: GalleryItemId::from_artifact_id(&second.id),
            artifact_id: second.id.clone(),
            artifact_kind: second.kind,
            source: second.source.clone(),
            primary_resource: second.primary_resource.clone(),
            assets: second.assets.clone(),
            metadata: second.metadata.clone(),
            safety_assessment: None,
            manual_safety_override: None,
            indexed_at_ms: 200,
        };

        artifacts.insert_artifact(first.clone()).await.unwrap();
        artifacts.insert_artifact(second).await.unwrap();
        gallery.upsert_item(first_item.clone()).await.unwrap();
        gallery.upsert_item(second_item).await.unwrap();
        let updated = gallery
            .set_safety_override(&first_item.id, Some(GallerySafetyOverride::Hidden))
            .await
            .unwrap();

        assert_eq!(
            updated.manual_safety_override,
            Some(GallerySafetyOverride::Hidden)
        );
        assert_eq!(
            gallery
                .query_items(GalleryQuery {
                    source_kind: Some(GallerySourceKind::Generation),
                    manual_safety_override: Some(GallerySafetyOverride::Hidden),
                    ..GalleryQuery::default()
                })
                .await
                .unwrap()
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            vec![first_item.id]
        );
        assert_eq!(
            gallery
                .query_items(GalleryQuery {
                    offset: 0,
                    limit: 1,
                    ..GalleryQuery::default()
                })
                .await
                .unwrap()[0]
                .artifact_id
                .as_str(),
            "artifact-2"
        );
    });
}

#[test]
fn generation_workflow_can_use_database_backed_payload_resource_artifact_and_gallery_ports() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let ports = DatabaseWorkflowPorts::new(connection.clone());
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-db");

        runtime
            .submit_generation_work(SubmitGenerationWork {
                batch_id: BatchId::new("batch-db"),
                job_id: job_id.clone(),
                request: GenerationWorkRequest::Image(GenerateImageRequest {
                    prompt: "1girl".to_owned(),
                    model: ImageModel::NaiDiffusion45Full,
                    ..Default::default()
                }),
                context: GenerationPlanContext::default(),
            })
            .await
            .unwrap();
        runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        let gallery = DatabaseGalleryIndex::new(connection);
        let items = gallery.query_items(GalleryQuery::default()).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].artifact_id.as_str(), "artifact:job-db:sample:0");
        assert_eq!(ports.generate_call_count(), 1);
    });
}

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
    ) -> nai_atelier_kernel::KernelResult<()> {
        self.payloads.save_submitted_payload(payload).await
    }

    async fn get_submitted_payload(
        &self,
        payload_ref: &JobPayloadRef,
    ) -> nai_atelier_kernel::KernelResult<Option<SubmittedGenerationPayload>> {
        self.payloads.get_submitted_payload(payload_ref).await
    }

    async fn save_prepared_payload(
        &self,
        payload: PreparedGenerationPayload,
    ) -> nai_atelier_kernel::KernelResult<()> {
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
        _request: nai_atelier_generation::GenerateImageStreamRequest,
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
