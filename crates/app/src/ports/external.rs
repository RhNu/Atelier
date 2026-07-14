use async_trait::async_trait;
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_kernel::KernelPreciseReferencePorts;
use atelier_precise_reference::{
    PreciseReferenceError, PreciseReferenceImage, PreciseReferenceResult,
};
use atelier_resource_catalog::ResourceRef;
use atelier_secrets::SecretStore;
use atelier_vibe::{
    EmbeddedVibeDocumentExtractor, EncodeVibeRequest, EncodedVibe, NovelAiVibeClient,
    VibeDomainResult, VibeResult,
};

use super::AppKernelPorts;

#[async_trait]
impl<S, F, E> NovelAiVibeClient for AppKernelPorts<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        self.novelai.encode_vibe(request).await
    }
}

#[async_trait]
impl<S, F, E> EmbeddedVibeDocumentExtractor for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    async fn extract_embedded_vibe_document_from_png(
        &self,
        png_bytes: &[u8],
    ) -> VibeDomainResult<Option<String>> {
        self.extractor
            .extract_embedded_vibe_document_from_png(png_bytes)
            .await
    }
}

#[async_trait]
impl<S, F, E> KernelPreciseReferencePorts for AppKernelPorts<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    async fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage> {
        let content = self
            .resource_reader
            .read_resource_base64(source)
            .await
            .map_err(|error| PreciseReferenceError::not_found(error.to_string()))?;
        let kind = self
            .resource_reader
            .read_resource_bytes(source)
            .await
            .map_err(|error| PreciseReferenceError::not_found(error.to_string()))?
            .kind;
        Ok(PreciseReferenceImage {
            kind,
            payload: content,
        })
    }
}
