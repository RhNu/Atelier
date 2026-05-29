use async_trait::async_trait;

use crate::{
    EncodeVibeRequest, EncodedVibe, VibeDocumentEntry, VibeDomainResult, VibeEncodeSettings,
    VibeEncodingRecord, VibeId, VibeResult, VibeSourceIdentity,
};

#[async_trait]
pub trait NovelAiVibeClient: Send + Sync {
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe>;
}

#[async_trait]
pub trait EmbeddedVibeDocumentExtractor: Send + Sync {
    async fn extract_embedded_vibe_document_from_png(
        &self,
        png_bytes: &[u8],
    ) -> VibeDomainResult<Option<String>>;
}

#[async_trait]
pub trait VibeRepository: Send + Sync {
    async fn insert_document(&self, entry: VibeDocumentEntry) -> VibeDomainResult<VibeId>;

    async fn get_document(&self, id: &VibeId) -> VibeDomainResult<Option<VibeDocumentEntry>>;

    async fn list_documents(
        &self,
        offset: usize,
        limit: usize,
    ) -> VibeDomainResult<Vec<VibeDocumentEntry>>;

    async fn count_documents(&self) -> VibeDomainResult<usize>;

    async fn find_cached_encoding(
        &self,
        source: &VibeSourceIdentity,
        settings: &VibeEncodeSettings,
    ) -> VibeDomainResult<Option<VibeEncodingRecord>>;

    async fn save_encoding(&self, record: VibeEncodingRecord) -> VibeDomainResult<()>;
}
