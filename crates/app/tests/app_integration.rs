use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_adapter_database::{DatabaseConnection, DatabaseResourceCatalogRepository};
use atelier_adapter_novelai::{NovelAiBridgeError, NovelAiClientFactory};
use atelier_adapter_storage_fs::workspace_database_path;
use atelier_app::WorkspaceSession;
use atelier_app_api::account::CreateApiKeyRequestDto;
use atelier_app_api::director::{DirectorToolDto, RunDirectorToolRequestDto};
use atelier_app_api::gallery::{
    GalleryQueryDto, GallerySafetyLabelDto, GallerySafetyRiskBandDto, GallerySourceKindDto,
};
use atelier_app_api::generation::{
    GenerateImageRequestDto, GenerateImageStreamRequestDto, GenerationEstimateRequestDto,
    GenerationPlanContextDto, GenerationWorkRequestDto, ImageModelDto, Img2ImgRequestDto,
    QueueDirectiveDto, StreamModeDto, SubmitGenerationBatchJobDto, SubmitGenerationBatchRequestDto,
    SubmitGenerationRequestDto,
};
use atelier_app_api::history::{
    DeleteGenerationHistoryBatchesRequestDto, GenerationHistoryBatchDetailDto,
    GenerationHistoryBatchRequestDto, GenerationHistoryQueryDto,
    RerunGenerationHistoryBatchRequestDto, RerunGenerationHistoryItemRequestDto, RunHistoryKindDto,
    RunHistoryQueryDto, RunHistoryStatusDto,
};
use atelier_app_api::resource::{
    GetResourceImageRequestDto, ImageInputDto, ImageResourceKindDto, ImportImageResourceRequestDto,
};
use atelier_app_api::settings::{ImageVariantSettingsDto, UpdateWorkspaceSettingsRequestDto};
use atelier_director::{
    DirectorClientError, DirectorResult, DirectorToolOutput, NovelAiDirectorClient,
    RunDirectorToolRequest,
};
use atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageStreamEvent, ImageStreamResult, NovelAiGenerationClient,
};
use atelier_resource_catalog::{ResourceCatalogRepository, VariantId};
use atelier_safety::{
    SafetyAssessment, SafetyModelScore, SafetyResult, SafetyScanInput, SafetyScanner,
};
use atelier_secrets::{
    SecretRecordId, SecretStore, SecretValue, SecretsResult, SubscriptionClient,
    SubscriptionResult, SubscriptionSummary,
};
use atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeResult};
use atelier_workspace::WorkspaceRoot;
use base64::Engine;
use futures_executor::block_on;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};

#[path = "app_integration/director_safety_history.rs"]
mod director_safety_history;
#[path = "app_integration/generation_outputs.rs"]
mod generation_outputs;
#[path = "app_integration/queue_recovery.rs"]
mod queue_recovery;

fn submit_request(batch_id: &str, job_id: &str, prompt: &str) -> SubmitGenerationRequestDto {
    SubmitGenerationRequestDto {
        batch_id: batch_id.to_owned(),
        job_id: job_id.to_owned(),
        work: GenerationWorkRequestDto::Image(GenerateImageRequestDto {
            prompt: prompt.to_owned(),
            model: ImageModelDto::NaiDiffusion45Full,
            ..GenerateImageRequestDto::default()
        }),
        context: GenerationPlanContextDto::default(),
    }
}

fn stream_submit_request(batch_id: &str, job_id: &str, prompt: &str) -> SubmitGenerationRequestDto {
    SubmitGenerationRequestDto {
        batch_id: batch_id.to_owned(),
        job_id: job_id.to_owned(),
        work: GenerationWorkRequestDto::Stream(GenerateImageStreamRequestDto {
            base: GenerateImageRequestDto {
                prompt: prompt.to_owned(),
                model: ImageModelDto::NaiDiffusion45Full,
                ..GenerateImageRequestDto::default()
            },
            stream: StreamModeDto::Sse,
        }),
        context: GenerationPlanContextDto::default(),
    }
}

fn submit_batch_request(
    batch_id: &str,
    jobs: &[(&str, &str, u32)],
) -> SubmitGenerationBatchRequestDto {
    SubmitGenerationBatchRequestDto {
        batch_id: batch_id.to_owned(),
        jobs: jobs
            .iter()
            .map(|(job_id, prompt, n_samples)| SubmitGenerationBatchJobDto {
                job_id: (*job_id).to_owned(),
                work: GenerationWorkRequestDto::Image(GenerateImageRequestDto {
                    prompt: (*prompt).to_owned(),
                    n_samples: *n_samples,
                    model: ImageModelDto::NaiDiffusion45Full,
                    ..GenerateImageRequestDto::default()
                }),
            })
            .collect(),
        context: GenerationPlanContextDto {
            request_count: u32::try_from(jobs.len()).unwrap_or(u32::MAX),
            ..GenerationPlanContextDto::default()
        },
    }
}

async fn test_app_with_image(
    temp: &tempfile::TempDir,
    image_bytes: Vec<u8>,
) -> WorkspaceSession<MemorySecretStore, RecordingFactory> {
    let app = WorkspaceSession::open_workspace_with_dependencies(
        temp.path().to_path_buf(),
        MemorySecretStore::default(),
        RecordingFactory::with_image_bytes(image_bytes),
    )
    .await
    .unwrap();
    app.account()
        .create_api_key(CreateApiKeyRequestDto {
            id: "main".to_owned(),
            display_name: "Main".to_owned(),
            secret: "active-secret".to_owned(),
        })
        .await
        .unwrap();
    app.account().set_active_api_key("main").await.unwrap();
    app
}

fn asset_roles_and_kinds(
    item: &atelier_app_api::gallery::GalleryItemDto,
) -> Vec<(&str, Option<&str>)> {
    item.assets
        .iter()
        .map(|asset| (asset.role.as_str(), asset.variant_kind.as_deref()))
        .collect()
}

fn variant_by_role(item: &atelier_app_api::gallery::GalleryItemDto, role: &str) -> String {
    item.assets
        .iter()
        .find(|asset| asset.role == role)
        .and_then(|asset| asset.resource.variant_id.clone())
        .unwrap()
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
    image_bytes: Arc<Vec<u8>>,
    generated_requests: Arc<Mutex<Vec<GenerateImageRequest>>>,
    director_requests: Arc<Mutex<Vec<RunDirectorToolRequest>>>,
    director_error: Arc<Mutex<Option<DirectorClientError>>>,
}

impl RecordingFactory {
    fn with_image_bytes(image_bytes: Vec<u8>) -> Self {
        Self {
            secrets: Arc::default(),
            image_bytes: Arc::new(image_bytes),
            generated_requests: Arc::default(),
            director_requests: Arc::default(),
            director_error: Arc::default(),
        }
    }

    fn with_director_error(error: DirectorClientError) -> Self {
        Self {
            director_error: Arc::new(Mutex::new(Some(error))),
            ..Self::default()
        }
    }

    fn secrets(&self) -> Vec<String> {
        self.secrets.lock().unwrap().clone()
    }

    fn generated_requests(&self) -> Vec<GenerateImageRequest> {
        self.generated_requests.lock().unwrap().clone()
    }

    fn director_requests(&self) -> Vec<RunDirectorToolRequest> {
        self.director_requests.lock().unwrap().clone()
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
            image_bytes: Arc::clone(&self.image_bytes),
            generated_requests: Arc::clone(&self.generated_requests),
            director_requests: Arc::clone(&self.director_requests),
            director_error: Arc::clone(&self.director_error),
        })
    }
}

#[derive(Clone)]
struct RecordingClient {
    image_bytes: Arc<Vec<u8>>,
    generated_requests: Arc<Mutex<Vec<GenerateImageRequest>>>,
    director_requests: Arc<Mutex<Vec<RunDirectorToolRequest>>>,
    director_error: Arc<Mutex<Option<DirectorClientError>>>,
}

#[derive(Default)]
struct RecordingSafetyScanner {
    inputs: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl RecordingSafetyScanner {
    fn inputs(&self) -> Vec<Vec<u8>> {
        self.inputs.lock().unwrap().clone()
    }
}

#[async_trait]
impl SafetyScanner for RecordingSafetyScanner {
    async fn scan_image(&self, input: SafetyScanInput) -> SafetyResult<SafetyAssessment> {
        self.inputs.lock().unwrap().push(input.bytes);
        SafetyAssessment::from_model_scores(
            input.resource,
            vec![
                SafetyModelScore::new("safe", 0.09)?,
                SafetyModelScore::new("nsfw", 0.91)?,
            ],
        )
        .map(|assessment| {
            assessment
                .with_scorer("mock_nsfw", Some("1"))
                .with_assessed_at_ms(123)
        })
    }
}

#[async_trait]
impl NovelAiGenerationClient for RecordingClient {
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        self.generated_requests.lock().unwrap().push(request);
        Ok(vec![GeneratedImage {
            bytes: (*self.image_bytes).clone(),
            mime_type: Some("image/png".to_owned()),
            seed: Some(42),
        }])
    }

    async fn generate_stream(
        &self,
        _request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        Ok(Box::pin(futures_util::stream::iter(vec![Ok(
            ImageStreamEvent {
                event_type: "final".to_owned(),
                sample_index: 0,
                step_index: Some(0),
                generation_id: 1,
                sigma: None,
                image: base64::engine::general_purpose::STANDARD
                    .encode(self.image_bytes.as_slice()),
            },
        )])))
    }
}

fn valid_png_bytes(width: u32, height: u32) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([
            u8::try_from(x % 256).unwrap(),
            u8::try_from(y % 256).unwrap(),
            255,
            255,
        ])
    }));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
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
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        self.director_requests.lock().unwrap().push(request);
        let director_error = self.director_error.lock().unwrap().clone();
        if let Some(error) = director_error {
            return Err(error);
        }
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
