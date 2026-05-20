use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_executor::block_on;
use nai_atelier_adapter_novelai::{NovelAiBridgeError, NovelAiClientFactory};
use nai_atelier_app::AppCommandHost;
use nai_atelier_app::GenerationWorkerCancel;
use nai_atelier_app_api::account::{
    CreateApiKeyRequestDto, ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto,
};
use nai_atelier_app_api::director::{DirectorToolDto, RunDirectorToolRequestDto};
use nai_atelier_app_api::event::{AppEventDto, AppEventKindDto, EventsSinceRequestDto};
use nai_atelier_app_api::gallery::{
    GalleryImageReferenceRequestDto, GalleryImageReferenceTargetDto, GalleryQueryDto,
    GallerySafetyOverrideDto, SetGallerySafetyOverrideRequestDto,
};
use nai_atelier_app_api::generation::{
    GenerateImageRequestDto, GenerationPlanContextDto, GenerationStatusQueryDto,
    GenerationWorkRequestDto, ImageModelDto, QueueDirectiveDto, RunGenerationJobRequestDto,
    SubmitGenerationRequestDto,
};
use nai_atelier_app_api::prompt::{
    DeletePromptChunkRequestDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
    PromptLexiconSearchQueryDto, UpsertPromptChunkRequestDto,
};
use nai_atelier_app_api::resource::{
    ImageInputDto, ImageResourceKindDto, ImportImageResourceRequestDto,
};
use nai_atelier_app_api::settings::{
    GenerationDefaultsDto, ImageVariantSettingsDto, UpdateWorkspaceSettingsRequestDto,
    WorkspaceSettingsDto,
};
use nai_atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, ExportVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
    VibeExportFormatDto, VibeModelDto,
};
use nai_atelier_app_api::workspace::OpenWorkspaceRequestDto;
use nai_atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use nai_atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationClientError,
    GenerationResult, ImageStreamResult, NovelAiGenerationClient,
};
use nai_atelier_secrets::{
    SecretRecordId, SecretStore, SecretValue, SecretsResult, SubscriptionClient,
    SubscriptionResult, SubscriptionSummary,
};
use nai_atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeResult};

#[test]
fn commands_require_open_workspace_and_close_session() {
    block_on(async {
        let host = test_host();
        let error = host.workspace_status().unwrap_err();
        assert_eq!(error.code, "workspace_not_open");

        let temp = tempfile::tempdir().unwrap();
        let status = host
            .open_workspace(OpenWorkspaceRequestDto {
                root: temp.path().to_path_buf(),
            })
            .await
            .unwrap();
        assert!(status.locked);
        let reopened = host
            .open_workspace(OpenWorkspaceRequestDto {
                root: temp.path().to_path_buf(),
            })
            .await
            .unwrap();
        assert!(reopened.locked);

        let invalid_root = tempfile::NamedTempFile::new().unwrap();
        let error = host
            .open_workspace(OpenWorkspaceRequestDto {
                root: invalid_root.path().to_path_buf(),
            })
            .await
            .unwrap_err();
        assert_ne!(error.code, "workspace_not_open");
        assert_eq!(host.workspace_status().unwrap().root, temp.path());

        let closed = host.close_workspace().unwrap();
        assert!(closed.was_open);
        let error = host.workspace_status().unwrap_err();
        assert_eq!(error.code, "workspace_not_open");
    });
}

#[test]
fn account_and_prompt_chunk_commands_share_session() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::default();
        let host = test_host_with_factory(factory.clone());
        open_workspace(&host, &temp).await;

        create_active_key(&host).await;
        assert!(factory.secrets().is_empty());
        let subscription = host
            .probe_api_key(ProbeApiKeyRequestDto {
                id: "main".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(subscription.anlas_balance, 100);
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        factory.clear();

        let hero = upsert_hero_chunk(&host).await;
        assert_eq!(
            host.get_prompt_chunk(GetPromptChunkRequestDto {
                chunk_id: None,
                key: Some("hero".to_owned()),
            })
            .await
            .unwrap()
            .content,
            "1girl"
        );
        assert_eq!(
            host.list_prompt_chunks(ListPromptChunksRequestDto {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap()
            .total,
            1
        );
        let companion = upsert_scene_chunk(&host).await;
        let referenced_delete = host
            .delete_prompt_chunk(DeletePromptChunkRequestDto {
                chunk_id: hero.chunk_id.clone(),
            })
            .await;
        assert_eq!(referenced_delete.unwrap_err().code, "prompt_conflict");

        assert!(
            host.delete_prompt_chunk(DeletePromptChunkRequestDto {
                chunk_id: companion.chunk_id,
            })
            .await
            .unwrap()
            .deleted
        );
        assert!(
            host.delete_prompt_chunk(DeletePromptChunkRequestDto {
                chunk_id: hero.chunk_id,
            })
            .await
            .unwrap()
            .deleted
        );
    });
}

#[test]
fn resource_import_command_is_available_through_facade() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let imported = host
            .import_image_resource(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::SourceImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap();

        assert!(imported.resource.id.starts_with("resource:import:source:"));
        assert_eq!(imported.resource.variant_id, None);
    });
}

#[test]
fn director_command_is_available_through_facade() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        let source = host
            .import_image_resource(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::SourceImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap()
            .resource;

        let result = host
            .run_director_tool(RunDirectorToolRequestDto {
                run_id: "run-1".to_owned(),
                tool: DirectorToolDto::Lineart,
                image: ImageInputDto::ResourceRef { resource: source },
                prompt: Some("clean lines".to_owned()),
                defry: Some(2),
                strict_mode: true,
            })
            .await
            .unwrap();

        assert_eq!(result.artifact_id, "director:run-1");
        assert_eq!(result.item.artifact_kind, "director_result");
        assert_eq!(result.resource.id, "resource:director:run-1");
    });
}

#[test]
fn settings_commands_persist_across_workspace_reopen() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let defaults = host.get_workspace_settings().await.unwrap();
        assert_eq!(defaults.image_variants.thumbnail_long_edge, 320);
        assert_eq!(defaults.image_variants.preview_long_edge, 1024);

        let updated = WorkspaceSettingsDto {
            generation: GenerationDefaultsDto {
                model: ImageModelDto::NaiDiffusion4Curated,
                n_samples: 2,
                ..GenerationDefaultsDto::default()
            },
            image_variants: ImageVariantSettingsDto {
                thumbnail_long_edge: 256,
                preview_long_edge: 768,
            },
        };
        assert_eq!(
            host.update_workspace_settings(UpdateWorkspaceSettingsRequestDto {
                settings: updated.clone(),
            })
            .await
            .unwrap(),
            updated
        );

        host.close_workspace().unwrap();
        open_workspace(&host, &temp).await;
        assert_eq!(host.get_workspace_settings().await.unwrap(), updated);

        let reset = host.reset_workspace_settings().await.unwrap();
        assert_eq!(reset.settings, WorkspaceSettingsDto::default());
        assert_eq!(
            host.get_workspace_settings().await.unwrap(),
            WorkspaceSettingsDto::default()
        );
    });
}

#[test]
fn generation_events_and_gallery_commands_share_session() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::default();
        let host = test_host_with_factory(factory.clone());
        open_workspace(&host, &temp).await;

        let missing_key = host
            .submit_generation(submit_request("batch-1", "job-1"))
            .await;
        assert_eq!(missing_key.unwrap_err().code, "missing_active_key");

        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;
        submit_and_run_generation(&host, "batch-1", "job-1").await;
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);

        factory.clear();
        submit_and_run_generation(&host, "batch-2", "job-2").await;
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);

        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job-2".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.batch_status.as_deref(), Some("succeeded"));
        assert_eq!(status.job_status.as_deref(), Some("succeeded"));

        let events = host
            .events_since(EventsSinceRequestDto {
                sequence: 0,
                limit: 100,
            })
            .unwrap();
        assert!(!events.items.is_empty());
        assert_eq!(
            events.next_sequence,
            events.items.last().map_or(0, |event| event.sequence)
        );

        let gallery = host
            .query_gallery(GalleryQueryDto {
                offset: 0,
                limit: 1,
                ..GalleryQueryDto::default()
            })
            .await
            .unwrap();
        assert_eq!(gallery.items.len(), 1);
        assert_eq!(gallery.total, 2);
        assert_eq!(gallery.items[0].artifact_kind, "generated_image");
        assert_eq!(
            host.query_gallery(GalleryQueryDto {
                offset: 0,
                limit: 10,
                artifact_kind: Some("generated_image".to_owned()),
                ..GalleryQueryDto::default()
            })
            .await
            .unwrap()
            .total,
            2
        );
        let item_id = gallery.items[0].item_id.clone();
        let reference = host
            .gallery_image_reference(GalleryImageReferenceRequestDto {
                item_id: item_id.clone(),
                target: GalleryImageReferenceTargetDto::PreciseReference,
            })
            .await
            .unwrap();
        assert_eq!(reference.item_id, item_id);
        assert_eq!(
            reference.target,
            GalleryImageReferenceTargetDto::PreciseReference
        );
        assert_eq!(reference.asset_role, "original");

        let overridden = host
            .set_gallery_safety_override(SetGallerySafetyOverrideRequestDto {
                item_id,
                manual_safety_override: Some(GallerySafetyOverrideDto::Hidden),
            })
            .await
            .unwrap();
        assert_eq!(
            overridden.manual_safety_override,
            Some(GallerySafetyOverrideDto::Hidden)
        );
    });
}

#[test]
fn generation_worker_drives_submitted_job_to_idle() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::default();
        let host = test_host_with_factory(factory.clone());
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;

        let directive = host
            .submit_generation(submit_request("batch-worker", "job-worker"))
            .await
            .unwrap();
        let final_directive = host
            .drive_generation_queue(directive, GenerationWorkerCancel::new())
            .await
            .unwrap();

        assert_eq!(final_directive, QueueDirectiveDto::Idle);
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job-worker".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.batch_status.as_deref(), Some("succeeded"));
        assert_eq!(status.job_status.as_deref(), Some("succeeded"));
        assert_eq!(
            host.query_gallery(GalleryQueryDto::default())
                .await
                .unwrap()
                .total,
            1
        );
    });
}

#[test]
fn generation_worker_advances_zero_delay() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host_with_factory(RecordingFactory::rate_limited_once());
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;

        let directive = host
            .submit_generation(submit_request("batch-delay", "job-delay"))
            .await
            .unwrap();
        let final_directive = host
            .drive_generation_queue(directive, GenerationWorkerCancel::new())
            .await
            .unwrap();

        assert_eq!(final_directive, QueueDirectiveDto::Idle);
    });
}

#[test]
fn generation_worker_cancel_stops_during_wait_without_advancing_queue() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host_with_factory(RecordingFactory::rate_limited_once());
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;

        host.submit_generation(submit_request("batch-cancel", "job-cancel"))
            .await
            .unwrap();
        let wait = host
            .run_generation_job(RunGenerationJobRequestDto {
                job_id: "job-cancel".to_owned(),
            })
            .await
            .unwrap();
        assert!(matches!(wait, QueueDirectiveDto::Wait { .. }));
        let cancel = GenerationWorkerCancel::new();
        cancel.cancel();

        let returned = host
            .drive_generation_queue(wait.clone(), cancel)
            .await
            .unwrap();

        assert_eq!(returned, wait);
        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job-cancel".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.batch_status.as_deref(), Some("waiting"));
    });
}

#[test]
fn event_subscription_receives_same_events_as_events_since() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        let received = Arc::new(Mutex::new(Vec::<AppEventDto>::new()));
        let received_events = Arc::clone(&received);
        host.subscribe_events(Arc::new(move |event| {
            received_events.lock().unwrap().push(event);
        }))
        .unwrap();

        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;
        let directive = host
            .submit_generation(submit_request("batch-events", "job-events"))
            .await
            .unwrap();
        host.drive_generation_queue(directive, GenerationWorkerCancel::new())
            .await
            .unwrap();

        let events = host
            .events_since(EventsSinceRequestDto {
                sequence: 0,
                limit: 100,
            })
            .unwrap()
            .items;
        let pushed = received.lock().unwrap().clone();

        assert_eq!(pushed, events);
        assert!(pushed.iter().any(|event| {
            matches!(
                &event.kind,
                AppEventKindDto::JobSucceeded { job_id, .. } if job_id == "job-events"
            )
        }));
    });
}

#[test]
fn prompt_lexicon_and_vibe_commands_are_available_through_facade() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        host.open_workspace(OpenWorkspaceRequestDto {
            root: temp.path().to_path_buf(),
        })
        .await
        .unwrap();
        host.create_api_key(CreateApiKeyRequestDto {
            id: "main".to_owned(),
            display_name: "Main".to_owned(),
            secret: "active-secret".to_owned(),
        })
        .await
        .unwrap();
        host.set_active_api_key(SetActiveApiKeyRequestDto {
            id: "main".to_owned(),
        })
        .await
        .unwrap();

        assert!(
            !host
                .prompt_lexicon_search(PromptLexiconSearchQueryDto {
                    query: "1girl".to_owned(),
                    limit: 5,
                })
                .unwrap()
                .items
                .is_empty()
        );

        let imported = host
            .import_vibe_document(ImportVibeDocumentRequestDto {
                file_name: "style.naiv4vibe".to_owned(),
                content: official_vibe("Style A"),
            })
            .await
            .unwrap();
        assert_eq!(imported.entries.len(), 1);
        let vibe_id = imported.entries[0].vibe_id.clone();

        let exported = host
            .export_vibe_document(ExportVibeDocumentRequestDto {
                vibe_ids: vec![vibe_id.clone()],
                format: VibeExportFormatDto::Naiv4vibe,
            })
            .await
            .unwrap();
        assert_eq!(exported.file_extension, "naiv4vibe");
        assert!(exported.content.contains("Style A"));

        let ensured = host
            .ensure_vibe_encoding(EnsureVibeEncodingRequestDto {
                vibe_id,
                source_sha256: "source-sha".to_owned(),
                image: "source-image-base64".to_owned(),
                model: VibeModelDto::NaiDiffusion45Full,
                information_extracted: 0.7,
            })
            .await
            .unwrap();
        assert!(!ensured.resource.id.is_empty());
    });
}

fn test_host() -> AppCommandHost<MemorySecretStore, RecordingFactory> {
    test_host_with_factory(RecordingFactory::default())
}

fn test_host_with_factory(
    factory: RecordingFactory,
) -> AppCommandHost<MemorySecretStore, RecordingFactory> {
    AppCommandHost::with_dependencies(MemorySecretStore::default(), factory)
}

async fn open_workspace(
    host: &AppCommandHost<MemorySecretStore, RecordingFactory>,
    temp: &tempfile::TempDir,
) {
    host.open_workspace(OpenWorkspaceRequestDto {
        root: temp.path().to_path_buf(),
    })
    .await
    .unwrap();
}

async fn create_active_key(host: &AppCommandHost<MemorySecretStore, RecordingFactory>) {
    host.create_api_key(CreateApiKeyRequestDto {
        id: "main".to_owned(),
        display_name: "Main".to_owned(),
        secret: "active-secret".to_owned(),
    })
    .await
    .unwrap();
    host.set_active_api_key(SetActiveApiKeyRequestDto {
        id: "main".to_owned(),
    })
    .await
    .unwrap();
}

async fn upsert_hero_chunk(
    host: &AppCommandHost<MemorySecretStore, RecordingFactory>,
) -> nai_atelier_app_api::prompt::PromptChunkDto {
    let hero = host
        .upsert_prompt_chunk(UpsertPromptChunkRequestDto {
            chunk_id: None,
            key: "hero".to_owned(),
            content: "1girl".to_owned(),
            category: Some("subject".to_owned()),
            description: None,
            preview: None,
        })
        .await
        .unwrap();
    assert_eq!(hero.key, "hero");
    hero
}

async fn upsert_scene_chunk(
    host: &AppCommandHost<MemorySecretStore, RecordingFactory>,
) -> nai_atelier_app_api::prompt::PromptChunkDto {
    host.upsert_prompt_chunk(UpsertPromptChunkRequestDto {
        chunk_id: None,
        key: "scene".to_owned(),
        content: "@chunk(hero), blue sky".to_owned(),
        category: None,
        description: None,
        preview: None,
    })
    .await
    .unwrap()
}

async fn submit_and_run_generation(
    host: &AppCommandHost<MemorySecretStore, RecordingFactory>,
    batch_id: &str,
    job_id: &str,
) {
    let directive = host
        .submit_generation(submit_request(batch_id, job_id))
        .await
        .unwrap();
    assert_eq!(
        directive,
        QueueDirectiveDto::StartJob {
            job_id: job_id.to_owned()
        }
    );
    host.run_generation_job(RunGenerationJobRequestDto {
        job_id: job_id.to_owned(),
    })
    .await
    .unwrap();
}

fn submit_request(batch_id: &str, job_id: &str) -> SubmitGenerationRequestDto {
    SubmitGenerationRequestDto {
        batch_id: batch_id.to_owned(),
        job_id: job_id.to_owned(),
        work: GenerationWorkRequestDto::Image(GenerateImageRequestDto {
            prompt: "@chunk(hero)".to_owned(),
            model: ImageModelDto::NaiDiffusion45Full,
            ..GenerateImageRequestDto::default()
        }),
        context: GenerationPlanContextDto::default(),
    }
}

const ENCODING_PAYLOAD: &str = "AQID";
const ENCODING_PAYLOAD_SHA256: &str =
    "b70035bb783a47bf61ac3ff70b005308e167ee984365690e638c1481b8ca2936";

fn official_vibe(name: &str) -> String {
    format!(
        r#"{{
  "identifier": "novelai-vibe-transfer",
  "version": 1,
  "type": "encoding",
  "id": "{ENCODING_PAYLOAD_SHA256}",
  "name": "{name}",
  "encodings": {{
    "v4-5full": {{
      "default": {{
        "encoding": "{ENCODING_PAYLOAD}",
        "params": {{ "information_extracted": 0.7 }}
      }}
    }}
  }}
}}"#
    )
}

#[derive(Clone, Default)]
struct MemorySecretStore {
    state: Arc<Mutex<Vec<(String, String)>>>,
}

#[async_trait]
impl SecretStore for MemorySecretStore {
    async fn write_secret(&self, id: &SecretRecordId, secret: SecretValue) -> SecretsResult<()> {
        self.state
            .lock()
            .unwrap()
            .push((id.as_str().to_owned(), secret.expose_secret().to_owned()));
        Ok(())
    }

    async fn read_secret(&self, id: &SecretRecordId) -> SecretsResult<SecretValue> {
        self.state
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == id.as_str())
            .map(|(_, value)| SecretValue::new(value.clone()))
            .ok_or_else(|| nai_atelier_secrets::SecretsError::missing_secret(id.as_str()))
    }

    async fn delete_secret(&self, id: &SecretRecordId) -> SecretsResult<bool> {
        let mut state = self.state.lock().unwrap();
        let before = state.len();
        state.retain(|(candidate, _)| candidate != id.as_str());
        Ok(state.len() != before)
    }
}

#[derive(Clone, Default)]
struct RecordingFactory {
    secrets: Arc<Mutex<Vec<String>>>,
    rate_limit_first_generate: bool,
    attempts: Arc<Mutex<u32>>,
}

impl RecordingFactory {
    fn rate_limited_once() -> Self {
        Self {
            secrets: Arc::default(),
            rate_limit_first_generate: true,
            attempts: Arc::default(),
        }
    }

    fn secrets(&self) -> Vec<String> {
        self.secrets.lock().unwrap().clone()
    }

    fn clear(&self) {
        self.secrets.lock().unwrap().clear();
    }
}

impl NovelAiClientFactory for RecordingFactory {
    type Client = RecordingClient;

    fn create_client(&self, secret: SecretValue) -> Result<Self::Client, NovelAiBridgeError> {
        self.secrets
            .lock()
            .unwrap()
            .push(secret.expose_secret().to_owned());
        Ok(RecordingClient {
            rate_limit_first_generate: self.rate_limit_first_generate,
            attempts: Arc::clone(&self.attempts),
        })
    }
}

#[derive(Clone)]
struct RecordingClient {
    rate_limit_first_generate: bool,
    attempts: Arc<Mutex<u32>>,
}

#[async_trait]
impl NovelAiGenerationClient for RecordingClient {
    async fn generate(
        &self,
        _request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        let should_rate_limit = {
            let mut attempts = self.attempts.lock().unwrap();
            let should_rate_limit = self.rate_limit_first_generate && *attempts == 0;
            *attempts += 1;
            should_rate_limit
        };
        if should_rate_limit {
            return Err(GenerationClientError::rate_limited(
                429,
                Some(Duration::from_millis(0)),
                "slow down",
            ));
        }
        Ok(vec![GeneratedImage {
            bytes: vec![1, 2, 3],
            mime_type: Some("image/png".to_owned()),
            seed: Some(42),
        }])
    }

    async fn generate_stream(
        &self,
        _request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        Ok(Box::pin(futures_util::stream::empty()))
    }
}

#[async_trait]
impl NovelAiVibeClient for RecordingClient {
    async fn encode_vibe(&self, _request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        Ok(EncodedVibe {
            payload: "encoded-vibe".to_owned(),
        })
    }
}

#[async_trait]
impl NovelAiDirectorClient for RecordingClient {
    async fn run_director_tool(
        &self,
        _request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        Ok(DirectorToolOutput {
            bytes: vec![4, 5, 6],
            mime_type: Some("image/png".to_owned()),
            seed: Some(7),
        })
    }
}

#[async_trait]
impl SubscriptionClient for RecordingClient {
    async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary> {
        Ok(SubscriptionSummary {
            anlas_balance: 100,
            is_opus: false,
            tier: 1,
            tier_name: "tablet".to_owned(),
            expires_at_ms: None,
        })
    }
}
