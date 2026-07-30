use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use atelier_adapter_novelai::{
    NovelAiBridgeError, NovelAiClientFactory, NovelAiEmbeddedVibeExtractor,
};
use atelier_app::AtelierRuntime;
use atelier_app::GenerationWorkerCancel;
use atelier_app_api::account::{
    CreateApiKeyRequestDto, ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto,
};
use atelier_app_api::director::{DirectorToolDto, RunDirectorToolRequestDto};
use atelier_app_api::event::{AppEventDto, AppEventKindDto, EventsSinceRequestDto};
use atelier_app_api::gallery::{
    DeleteGalleryItemsRequestDto, GalleryImageReferenceRequestDto, GalleryImageReferenceTargetDto,
    GalleryQueryDto, GallerySafetyLabelDto, GallerySafetyOverrideDto,
    SetGallerySafetyOverrideRequestDto,
};
use atelier_app_api::generation::{
    GenerateImageRequestDto, GenerationDraftCharacterPositionModeDto, GenerationDraftDto,
    GenerationDraftSeedModeDto, GenerationDraftVibeDto, GenerationDraftVibeSlotDto,
    GenerationPlanContextDto, GenerationStatusQueryDto, GenerationWorkRequestDto, ImageModelDto,
    QueueDirectiveDto, RunGenerationJobRequestDto, SaveGenerationDraftRequestDto,
    SubmitGenerationRequestDto,
};
use atelier_app_api::history::{RunHistoryOutputStateDto, RunHistoryQueryDto};
use atelier_app_api::prompt::{
    CompileGenerationPromptRequestDto, DeletePromptChunkRequestDto, DeletePromptPresetRequestDto,
    GetPromptChunkRequestDto, ListPromptChunksRequestDto, ListPromptPresetsRequestDto,
    PromptChunkDto, PromptPresetBehaviorDto, PromptPresetDto, PromptPresetKindDto,
    UpsertPromptChunkRequestDto, UpsertPromptPresetRequestDto,
};
use atelier_app_api::resource::{
    GetResourceImageRequestDto, ImageInputDto, ImageResourceKindDto, ImportImageResourceRequestDto,
    ReleaseImportedImageResourcesRequestDto, ResourceRefDto,
};
use atelier_app_api::settings::{
    GenerationDefaultsDto, GlobalFrontendSettingsDto, GlobalGallerySettingsDto,
    GlobalSafetySettingsDto, ImageVariantSettingsDto, UpdateGlobalSettingsRequestDto,
    UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, ExportVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
    ListVibeDocumentsRequestDto, RenameVibeDocumentRequestDto, SetVibeDocumentHiddenRequestDto,
    VibeExportFormatDto, VibeModelDto,
};
use atelier_app_api::workspace::OpenWorkspaceRequestDto;
use atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use atelier_generation::{
    GenerateImageRequest, GenerateImageResult, GenerateImageStreamRequest,
    GenerateImageStreamResult, GeneratedImage, GeneratedImageMetadata, GenerationClientError,
    GenerationResult, NovelAiGenerationClient, ParsedGeneratedImageMetadata,
};
use atelier_secrets::{
    SecretRecordId, SecretStore, SecretValue, SecretsResult, SubscriptionClient,
    SubscriptionResult, SubscriptionSummary,
};
use atelier_settings::{
    GlobalSettings, GlobalSettingsRepository, GlobalSettingsService, SettingsResult,
};
use atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeResult};
use futures_executor::block_on;

#[path = "command_facade/danbooru_commands.rs"]
mod danbooru_commands;
#[path = "command_facade/resource_preview_commands.rs"]
mod resource_preview_commands;
#[path = "command_facade/session_commands.rs"]
mod session_commands;
#[path = "command_facade/worker_events.rs"]
mod worker_events;

fn test_host() -> AtelierRuntime<MemorySecretStore, RecordingFactory> {
    test_host_with_factory(RecordingFactory::default())
}

fn test_host_with_factory(
    factory: RecordingFactory,
) -> AtelierRuntime<MemorySecretStore, RecordingFactory> {
    AtelierRuntime::with_dependencies(MemorySecretStore::default(), factory)
}

fn test_host_with_global_settings(
    settings: GlobalSettings,
) -> AtelierRuntime<MemorySecretStore, RecordingFactory> {
    AtelierRuntime::with_global_settings_dependencies_extractor_and_safety_scanner(
        GlobalSettingsService::new(Arc::new(MemoryGlobalSettingsRepository {
            settings: Mutex::new(settings),
        })),
        MemorySecretStore::default(),
        RecordingFactory::default(),
        NovelAiEmbeddedVibeExtractor,
        None,
    )
}

async fn open_workspace(
    host: &AtelierRuntime<MemorySecretStore, RecordingFactory>,
    temp: &tempfile::TempDir,
) {
    host.open_workspace(OpenWorkspaceRequestDto {
        root: temp.path().to_path_buf(),
    })
    .await
    .unwrap();
}

async fn create_active_key(host: &AtelierRuntime<MemorySecretStore, RecordingFactory>) {
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

struct MemoryGlobalSettingsRepository {
    settings: Mutex<GlobalSettings>,
}

#[async_trait]
impl GlobalSettingsRepository for MemoryGlobalSettingsRepository {
    async fn get_global_settings(&self) -> SettingsResult<GlobalSettings> {
        Ok(self.settings.lock().unwrap().clone())
    }

    async fn save_global_settings(&self, settings: GlobalSettings) -> SettingsResult<()> {
        *self.settings.lock().unwrap() = settings;
        Ok(())
    }
}

async fn upsert_hero_chunk(
    host: &AtelierRuntime<MemorySecretStore, RecordingFactory>,
) -> atelier_app_api::prompt::PromptChunkDto {
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
    host: &AtelierRuntime<MemorySecretStore, RecordingFactory>,
) -> atelier_app_api::prompt::PromptChunkDto {
    host.upsert_prompt_chunk(UpsertPromptChunkRequestDto {
        chunk_id: None,
        key: "scene".to_owned(),
        content: "$chunk(hero), blue sky".to_owned(),
        category: None,
        description: None,
        preview: None,
    })
    .await
    .unwrap()
}

async fn submit_and_run_generation(
    host: &AtelierRuntime<MemorySecretStore, RecordingFactory>,
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
            prompt: "$chunk(hero)".to_owned(),
            model: ImageModelDto::NaiDiffusion45Full,
            ..GenerateImageRequestDto::default()
        }),
        context: GenerationPlanContextDto::default(),
    }
}

const ENCODING_PAYLOAD: &str = "AQID";
const ENCODING_PAYLOAD_SHA256: &str =
    "b70035bb783a47bf61ac3ff70b005308e167ee984365690e638c1481b8ca2936";
const IMAGE_PAYLOAD: &str = "data:image/png;base64,AQID";
const IMAGE_PAYLOAD_SHA256: &str =
    "8a37c75fc31bfff9652678f50164cf27b31c787b8a946f87f1cd3cbf61365db1";

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

fn official_image_vibe(name: &str) -> String {
    format!(
        r#"{{
  "identifier": "novelai-vibe-transfer",
  "version": 1,
  "type": "image",
  "id": "{IMAGE_PAYLOAD_SHA256}",
  "name": "{name}",
  "image": "{IMAGE_PAYLOAD}",
  "thumbnail": "{IMAGE_PAYLOAD}",
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
            .ok_or_else(|| atelier_secrets::SecretsError::missing_secret(id.as_str()))
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
    ) -> GenerationResult<GenerateImageResult> {
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
        Ok(GenerateImageResult {
            resolved_seed: 42,
            images: vec![GeneratedImage {
                bytes: vec![1, 2, 3],
                mime_type: Some("image/png".to_owned()),
                metadata: GeneratedImageMetadata::Parsed(ParsedGeneratedImageMetadata {
                    prompt: Some("1girl".to_owned()),
                    negative_prompt: None,
                    seed: Some(42),
                    metadata_json: r#"{"seed":42}"#.to_owned(),
                    warnings: Vec::new(),
                }),
            }],
        })
    }

    async fn generate_stream(
        &self,
        _request: GenerateImageStreamRequest,
    ) -> GenerationResult<GenerateImageStreamResult> {
        Ok(GenerateImageStreamResult {
            resolved_seed: 42,
            stream: Box::pin(futures_util::stream::empty()),
        })
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
