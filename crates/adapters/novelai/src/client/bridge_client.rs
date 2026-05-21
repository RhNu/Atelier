use super::{
    DirectorResult, DirectorToolOutput, EncodeVibeRequest, EncodedVibe, GenerateImageRequest,
    GenerateImageStreamRequest, GeneratedImage, GenerationResult, ImageStreamResult,
    NovelAiBridgeAdapter, NovelAiDirectorClient, NovelAiGenerationClient, NovelAiVibeClient,
    RunDirectorToolRequest, StreamExt, SubscriptionClient, SubscriptionResult, SubscriptionSummary,
    VibeResult, async_trait, bridge, from_bridge_generated_image, from_bridge_stream_chunk,
    from_bridge_subscription, map_bridge_error, map_director_error, map_generation_error,
    map_subscription_error, map_vibe_error, to_bridge_director_request,
    to_bridge_encode_vibe_request, to_bridge_generate_request, to_bridge_stream_request,
};

#[async_trait]
impl<T> NovelAiGenerationClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        self.client
            .generate(to_bridge_generate_request(request))
            .await
            .map(|images| {
                images
                    .into_iter()
                    .map(from_bridge_generated_image)
                    .collect()
            })
            .map_err(|error| map_generation_error(map_bridge_error(error)))
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        let stream = self
            .client
            .generate_stream(to_bridge_stream_request(request))
            .await
            .map_err(|error| map_generation_error(map_bridge_error(error)))?;
        Ok(Box::pin(stream.map(|item| {
            item.map(from_bridge_stream_chunk)
                .map_err(|error| map_generation_error(map_bridge_error(error)))
        })))
    }
}

#[async_trait]
impl<T> NovelAiVibeClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        self.client
            .encode_vibe(to_bridge_encode_vibe_request(request))
            .await
            .map(|payload| EncodedVibe { payload })
            .map_err(|error| map_vibe_error(map_bridge_error(error)))
    }
}

#[async_trait]
impl<T> NovelAiDirectorClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        let request = request.normalize_for_tool()?;
        self.client
            .run_director_tool(to_bridge_director_request(request))
            .await
            .map(|image| DirectorToolOutput {
                bytes: image.bytes,
                mime_type: image.mime_type,
                seed: image.seed,
            })
            .map_err(|error| map_director_error(map_bridge_error(error)))
    }
}

#[async_trait]
impl<T> SubscriptionClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary> {
        self.client
            .get_subscription()
            .await
            .map(from_bridge_subscription)
            .map_err(|error| map_subscription_error(map_bridge_error(error)))
    }
}
