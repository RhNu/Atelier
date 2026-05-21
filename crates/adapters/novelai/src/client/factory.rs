use super::{
    NovelAiBridgeAdapter, NovelAiBridgeConfig, NovelAiBridgeError, NovelAiDirectorClient,
    NovelAiGenerationClient, NovelAiVibeClient, SecretValue, SubscriptionClient, bridge,
};

pub trait NovelAiClientFactory: Clone + Send + Sync {
    type Client: NovelAiGenerationClient
        + NovelAiVibeClient
        + NovelAiDirectorClient
        + SubscriptionClient
        + Send
        + Sync;

    /// Creates a `NovelAI` client for one resolved secret value.
    ///
    /// # Errors
    /// Returns an error when the client cannot be constructed from the secret.
    fn create_client(&self, secret: SecretValue) -> Result<Self::Client, NovelAiBridgeError>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ReqwestNovelAiClientFactory {
    timeout_ms: u64,
}

impl Default for ReqwestNovelAiClientFactory {
    fn default() -> Self {
        Self {
            timeout_ms: bridge::DEFAULT_TIMEOUT_MS,
        }
    }
}

impl ReqwestNovelAiClientFactory {
    #[must_use]
    pub const fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl NovelAiClientFactory for ReqwestNovelAiClientFactory {
    type Client = NovelAiBridgeAdapter<bridge::ReqwestTransport>;

    fn create_client(&self, secret: SecretValue) -> Result<Self::Client, NovelAiBridgeError> {
        NovelAiBridgeAdapter::new(NovelAiBridgeConfig {
            api_key: secret.expose_secret().to_owned(),
            timeout_ms: self.timeout_ms,
        })
    }
}
