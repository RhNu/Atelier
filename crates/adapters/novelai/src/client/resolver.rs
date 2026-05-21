use super::{
    DirectorResult, DirectorToolOutput, EncodeVibeRequest, EncodedVibe, GenerateImageRequest,
    GenerateImageStreamRequest, GeneratedImage, GenerationResult, ImageStreamResult,
    NovelAiBridgeError, NovelAiClientFactory, NovelAiDirectorClient, NovelAiGenerationClient,
    NovelAiVibeClient, ReqwestNovelAiClientFactory, RunDirectorToolRequest, SecretResolver,
    SubscriptionClient, SubscriptionResult, SubscriptionSummary, VibeResult, async_trait,
    map_director_error, map_generation_error, map_secrets_error, map_subscription_error,
    map_vibe_error,
};

#[derive(Clone, Debug)]
pub struct ResolverBackedNovelAiAdapter<R, F = ReqwestNovelAiClientFactory> {
    resolver: R,
    factory: F,
}

impl<R, F> ResolverBackedNovelAiAdapter<R, F> {
    #[must_use]
    pub const fn new(resolver: R, factory: F) -> Self {
        Self { resolver, factory }
    }
}

impl<R, F> ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn create_client(&self) -> Result<F::Client, NovelAiBridgeError> {
        let secret = self
            .resolver
            .resolve_active_secret()
            .await
            .map_err(|error| map_secrets_error(&error))?;
        self.factory.create_client(secret)
    }
}

#[async_trait]
impl<R, F> NovelAiGenerationClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        self.create_client()
            .await
            .map_err(map_generation_error)?
            .generate(request)
            .await
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        self.create_client()
            .await
            .map_err(map_generation_error)?
            .generate_stream(request)
            .await
    }
}

#[async_trait]
impl<R, F> NovelAiVibeClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        self.create_client()
            .await
            .map_err(map_vibe_error)?
            .encode_vibe(request)
            .await
    }
}

#[async_trait]
impl<R, F> NovelAiDirectorClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        self.create_client()
            .await
            .map_err(map_director_error)?
            .run_director_tool(request)
            .await
    }
}

#[async_trait]
impl<R, F> SubscriptionClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary> {
        self.create_client()
            .await
            .map_err(map_subscription_error)?
            .get_subscription()
            .await
    }
}
