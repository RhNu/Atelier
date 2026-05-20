use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;

use crate::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationClientError,
    ImageStreamEvent,
};

pub type GenerationResult<T> = Result<T, GenerationClientError>;
pub type ImageStreamResult =
    Pin<Box<dyn Stream<Item = GenerationResult<ImageStreamEvent>> + Send + 'static>>;

#[async_trait]
pub trait NovelAiGenerationClient: Send + Sync {
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>>;

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult>;
}
