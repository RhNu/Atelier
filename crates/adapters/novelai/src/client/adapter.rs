use super::{
    EmbeddedVibeDocumentExtractor, NovelAiBridgeConfig, NovelAiBridgeError, VibeDomainResult,
    VibeError, async_trait, bridge, map_bridge_error,
};
use std::sync::Arc;

pub struct NovelAiBridgeAdapter<T: bridge::Transport = bridge::ReqwestTransport> {
    pub(super) client: bridge::Client<T>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct NovelAiEmbeddedVibeExtractor;

#[async_trait]
impl EmbeddedVibeDocumentExtractor for NovelAiEmbeddedVibeExtractor {
    async fn extract_embedded_vibe_document_from_png(
        &self,
        png_bytes: &[u8],
    ) -> VibeDomainResult<Option<String>> {
        bridge::extract_embedded_vibe_document_from_png_bytes(png_bytes)
            .map_err(|error| VibeError::invalid_document(error.to_string()))
    }
}

impl NovelAiBridgeAdapter<bridge::ReqwestTransport> {
    /// Creates a bridge-backed adapter from an explicit API key.
    ///
    /// This deliberately does not read `NOVELAI_API_KEY`; secret resolution is
    /// owned by the future secrets/keyring adapters.
    /// # Errors
    /// Returns an error when the bridge client rejects the supplied configuration.
    pub fn new(config: NovelAiBridgeConfig) -> Result<Self, NovelAiBridgeError> {
        let options = bridge::ClientOptions {
            api_key_source: bridge::ApiKeySource::Inline {
                value: config.api_key.into(),
            },
            timeout_ms: config.timeout_ms,
            ..bridge::ClientOptions::default()
        };
        let tokenizers = bridge::Tokenizers::bundled()
            .map_err(bridge::BridgeError::from)
            .map_err(map_bridge_error)?;
        bridge::Client::new(options)
            .map(|client| client.with_tokenizers(Arc::new(tokenizers)))
            .map(Self::from_client)
            .map_err(map_bridge_error)
    }
}

impl<T: bridge::Transport> NovelAiBridgeAdapter<T> {
    #[must_use]
    pub const fn from_client(client: bridge::Client<T>) -> Self {
        Self { client }
    }
}
