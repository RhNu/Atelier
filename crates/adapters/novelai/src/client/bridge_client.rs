use super::{
    DirectorResult, DirectorToolOutput, EncodeVibeRequest, EncodedVibe, GenerateImageRequest,
    GenerateImageResult, GenerateImageStreamRequest, GenerateImageStreamResult,
    GeneratedImageMetadata, GeneratedImageMetadataInspector, GenerationResult,
    NovelAiBridgeAdapter, NovelAiDirectorClient, NovelAiGenerationClient, NovelAiVibeClient,
    RunDirectorToolRequest, StreamExt, SubscriptionClient, SubscriptionResult, SubscriptionSummary,
    VibeResult, async_trait, bridge, from_bridge_generated_image,
    from_bridge_generated_image_metadata, from_bridge_stream_chunk, from_bridge_subscription,
    map_bridge_error, map_director_error, map_generation_error, map_subscription_error,
    map_vibe_error, to_bridge_director_request, to_bridge_encode_vibe_request,
    to_bridge_generate_request, to_bridge_stream_request,
};

#[async_trait]
impl<T> NovelAiGenerationClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport + bridge::StreamingTransport,
{
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<GenerateImageResult> {
        let request = to_bridge_generate_request(request)
            .map_err(|error| map_generation_error(map_bridge_error(error)))?;
        self.client
            .generate(request)
            .await
            .map(|result| GenerateImageResult {
                resolved_seed: result.resolved_seed,
                images: result
                    .images
                    .into_iter()
                    .map(from_bridge_generated_image)
                    .collect(),
            })
            .map_err(|error| map_generation_error(map_bridge_error(error)))
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<GenerateImageStreamResult> {
        let request = to_bridge_stream_request(request)
            .map_err(|error| map_generation_error(map_bridge_error(error)))?;
        let stream = self
            .client
            .generate_stream(request)
            .await
            .map_err(|error| map_generation_error(map_bridge_error(error)))?;
        Ok(GenerateImageStreamResult {
            resolved_seed: stream.resolved_seed,
            stream: Box::pin(stream.map(|item| {
                item.map(from_bridge_stream_chunk)
                    .map_err(|error| map_generation_error(map_bridge_error(error)))
            })),
        })
    }
}

impl<T> GeneratedImageMetadataInspector for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    fn inspect_generated_image_metadata(
        &self,
        bytes: &[u8],
        mime_type: Option<&str>,
    ) -> GeneratedImageMetadata {
        from_bridge_generated_image_metadata(bridge::inspect_generated_image_metadata(
            bytes, mime_type,
        ))
    }
}

#[async_trait]
impl<T> NovelAiVibeClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        let request = to_bridge_encode_vibe_request(&request)
            .map_err(|error| map_vibe_error(map_bridge_error(error)))?;
        self.client
            .encode_vibe(request)
            .await
            .map(|payload| EncodedVibe {
                payload: payload.to_base64(),
            })
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
        let request = to_bridge_director_request(request)
            .map_err(|error| map_director_error(map_bridge_error(error)))?;
        self.client
            .run_director_tool(request)
            .await
            .map(|image| {
                let seed = image.seed();
                DirectorToolOutput {
                    bytes: image.bytes,
                    mime_type: image.mime_type,
                    seed,
                }
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
