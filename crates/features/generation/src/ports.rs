use std::{
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures_core::Stream;

use crate::{
    GenerateImageRequest, GenerateImageResult, GenerateImageStreamRequest, GeneratedImageMetadata,
    GenerationClientError, ImageStreamEvent,
};

pub type GenerationResult<T> = Result<T, GenerationClientError>;
#[async_trait]
pub trait CancellableImageStream:
    Stream<Item = GenerationResult<ImageStreamEvent>> + Send + Unpin + 'static
{
    async fn cancel(self: Box<Self>);
}

pub type ImageStreamResult = Box<dyn CancellableImageStream>;

struct PassiveImageStream {
    inner: Pin<Box<dyn Stream<Item = GenerationResult<ImageStreamEvent>> + Send + 'static>>,
}

impl Stream for PassiveImageStream {
    type Item = GenerationResult<ImageStreamEvent>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context)
    }
}

#[async_trait]
impl CancellableImageStream for PassiveImageStream {
    async fn cancel(self: Box<Self>) {}
}

#[must_use]
pub fn passive_image_stream(
    stream: Pin<Box<dyn Stream<Item = GenerationResult<ImageStreamEvent>> + Send + 'static>>,
) -> ImageStreamResult {
    Box::new(PassiveImageStream { inner: stream })
}

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
