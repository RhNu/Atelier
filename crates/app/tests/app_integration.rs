use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_executor::block_on;
use nai_atelier_adapter_novelai::{NovelAiBridgeError, NovelAiClientFactory};
use nai_atelier_app::AtelierApp;
use nai_atelier_app_api::account::CreateApiKeyRequestDto;
use nai_atelier_app_api::gallery::GalleryQueryDto;
use nai_atelier_app_api::generation::{
    GenerateImageRequestDto, GenerationPlanContextDto, GenerationWorkRequestDto, ImageModelDto,
    QueueDirectiveDto, SubmitGenerationRequestDto,
};
use nai_atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use nai_atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageStreamResult, NovelAiGenerationClient,
};
use nai_atelier_secrets::{
    SecretRecordId, SecretStore, SecretValue, SecretsResult, SubscriptionClient,
    SubscriptionResult, SubscriptionSummary,
};
use nai_atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeResult};

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
                .items
                .len(),
            1
        );
        assert!(!app.events().events_since(0, 100).is_empty());
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
}

impl RecordingFactory {
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
        Ok(RecordingClient)
    }
}

#[derive(Clone)]
struct RecordingClient;

#[async_trait]
impl NovelAiGenerationClient for RecordingClient {
    async fn generate(
        &self,
        _request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
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
