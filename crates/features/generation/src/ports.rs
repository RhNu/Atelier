use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;

use crate::{
    GenerateImageRequest, GenerateImageResult, GenerateImageStreamRequest, GeneratedImageMetadata,
    GenerationClientError, ImageStreamEvent,
};

pub type GenerationResult<T> = Result<T, GenerationClientError>;
pub type ImageStreamResult =
    Pin<Box<dyn Stream<Item = GenerationResult<ImageStreamEvent>> + Send + 'static>>;

pub struct GenerateImageStreamResult {
    pub resolved_seed: i64,
    pub stream: ImageStreamResult,
}

#[async_trait]
pub trait NovelAiGenerationClient: Send + Sync {
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<GenerateImageResult>;

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<GenerateImageStreamResult>;
}

pub trait GeneratedImageMetadataInspector: Send + Sync {
    fn inspect_generated_image_metadata(
        &self,
        bytes: &[u8],
        mime_type: Option<&str>,
    ) -> GeneratedImageMetadata;
}
