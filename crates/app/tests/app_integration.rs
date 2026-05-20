use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine;
use futures_executor::block_on;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use nai_atelier_adapter_database::{DatabaseConnection, DatabaseResourceCatalogRepository};
use nai_atelier_adapter_novelai::{NovelAiBridgeError, NovelAiClientFactory};
use nai_atelier_adapter_storage_fs::workspace_database_path;
use nai_atelier_app::AtelierApp;
use nai_atelier_app_api::account::CreateApiKeyRequestDto;
use nai_atelier_app_api::gallery::GalleryQueryDto;
use nai_atelier_app_api::generation::{
    GenerateImageRequestDto, GenerateImageStreamRequestDto, GenerationPlanContextDto,
    GenerationWorkRequestDto, ImageModelDto, QueueDirectiveDto, StreamModeDto,
    SubmitGenerationRequestDto,
};
use nai_atelier_app_api::settings::{ImageVariantSettingsDto, UpdateWorkspaceSettingsRequestDto};
use nai_atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use nai_atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageStreamEvent, ImageStreamResult, NovelAiGenerationClient,
};
use nai_atelier_resource_catalog::{ResourceCatalogRepository, VariantId};
use nai_atelier_secrets::{
    SecretRecordId, SecretStore, SecretValue, SecretsResult, SubscriptionClient,
    SubscriptionResult, SubscriptionSummary,
};
use nai_atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeResult};
use nai_atelier_workspace::WorkspaceRoot;

#[test]
fn open_workspace_initializes_lexicon_and_generation_is_explicitly_driven() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let secrets = MemorySecretStore::default();
        let factory = RecordingFactory::default();
        let app = AtelierApp::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets,
            factory.clone(),
        )
        .await
        .unwrap();

        assert!(
            !app.prompt()
                .lexicon_search("1girl", 5)
                .unwrap()
                .items
                .is_empty()
        );

        let missing_key = app
            .generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap_err();
        assert_eq!(missing_key.code(), "missing_active_key");

        app.account()
            .create_api_key(CreateApiKeyRequestDto {
                id: "main".to_owned(),
                display_name: "Main".to_owned(),
                secret: "active-secret".to_owned(),
            })
            .await
            .unwrap();
        app.account().set_active_api_key("main").await.unwrap();
        app.prompt()
            .upsert_chunk(nai_atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        let directive = app
            .generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        assert_eq!(
            directive,
            QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned()
            }
        );
        assert!(factory.secrets().is_empty());

        app.generation().run_job("job-1").await.unwrap();

        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        assert_eq!(
            app.gallery()
                .query(GalleryQueryDto::default())
                .await
                .unwrap()
                .items[0]
                .assets
                .iter()
                .map(|asset| asset.role.as_str())
                .collect::<Vec<_>>(),
            vec!["original"]
        );
    });
}

#[test]
fn valid_generated_images_get_best_effort_gallery_variants() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let secrets = MemorySecretStore::default();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let app = AtelierApp::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets,
            factory,
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
        app.prompt()
            .upsert_chunk(nai_atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        assert_eq!(gallery.items.len(), 1);
        let item = &gallery.items[0];
        assert_eq!(
            item.assets
                .iter()
                .map(|asset| (asset.role.as_str(), asset.variant_kind.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("original", Some("original")),
                ("thumbnail", Some("thumbnail")),
                ("preview", Some("preview")),
                ("sanitized", Some("sanitized")),
                ("export", Some("export")),
            ]
        );
        assert!(!app.events().events_since(0, 100).is_empty());
    });
}

#[test]
fn valid_streamed_images_get_best_effort_gallery_variants() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(2, 1)).await;
        app.prompt()
            .upsert_chunk(nai_atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(stream_submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        assert_eq!(gallery.items.len(), 1);
        assert_eq!(
            asset_roles_and_kinds(&gallery.items[0]),
            vec![
                ("original", Some("original")),
                ("thumbnail", Some("thumbnail")),
                ("preview", Some("preview")),
                ("sanitized", Some("sanitized")),
                ("export", Some("export")),
            ]
        );
    });
}

#[test]
fn updated_variant_settings_drive_generated_variant_dimensions() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(1200, 600)).await;
        let mut settings = app.settings().get().await.unwrap();
        settings.image_variants = ImageVariantSettingsDto {
            thumbnail_long_edge: 160,
            preview_long_edge: 640,
        };
        app.settings()
            .update(UpdateWorkspaceSettingsRequestDto { settings })
            .await
            .unwrap();
        app.prompt()
            .upsert_chunk(nai_atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        let item = &gallery.items[0];
        let repository = DatabaseResourceCatalogRepository::new(
            DatabaseConnection::open(workspace_database_path(&WorkspaceRoot::new(
                temp.path().to_path_buf(),
            )))
            .unwrap(),
        );
        let thumbnail = variant_by_role(item, "thumbnail");
        let preview = variant_by_role(item, "preview");

        let thumbnail = repository
            .get_variant(&VariantId::new(thumbnail))
            .await
            .unwrap()
            .unwrap();
        let preview = repository
            .get_variant(&VariantId::new(preview))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(thumbnail.metadata.mime_type.as_deref(), Some("image/webp"));
        assert_eq!(
            (thumbnail.metadata.width, thumbnail.metadata.height),
            (Some(160), Some(80))
        );
        assert_eq!(preview.metadata.mime_type.as_deref(), Some("image/webp"));
        assert_eq!(
            (preview.metadata.width, preview.metadata.height),
            (Some(640), Some(320))
        );
    });
}

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

async fn test_app_with_image(
    temp: &tempfile::TempDir,
    image_bytes: Vec<u8>,
) -> AtelierApp<MemorySecretStore, RecordingFactory> {
    let app = AtelierApp::open_workspace_with_dependencies(
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
    item: &nai_atelier_app_api::gallery::GalleryItemDto,
) -> Vec<(&str, Option<&str>)> {
    item.assets
        .iter()
        .map(|asset| (asset.role.as_str(), asset.variant_kind.as_deref()))
        .collect()
}

fn variant_by_role(item: &nai_atelier_app_api::gallery::GalleryItemDto, role: &str) -> String {
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
    image_bytes: Arc<Vec<u8>>,
}

impl RecordingFactory {
    fn with_image_bytes(image_bytes: Vec<u8>) -> Self {
        Self {
            secrets: Arc::default(),
            image_bytes: Arc::new(image_bytes),
        }
    }

    fn secrets(&self) -> Vec<String> {
        self.secrets.lock().unwrap().clone()
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
        })
    }
}

#[derive(Clone)]
struct RecordingClient {
    image_bytes: Arc<Vec<u8>>,
}

#[async_trait]
impl NovelAiGenerationClient for RecordingClient {
    async fn generate(
        &self,
        _request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
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
