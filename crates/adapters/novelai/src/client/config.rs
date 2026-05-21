use super::bridge;

#[derive(Clone, PartialEq, Eq)]
pub struct NovelAiBridgeConfig {
    pub api_key: String,
    pub timeout_ms: u64,
}

impl std::fmt::Debug for NovelAiBridgeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NovelAiBridgeConfig")
            .field("api_key", &"<redacted>")
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

impl NovelAiBridgeConfig {
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            timeout_ms: bridge::DEFAULT_TIMEOUT_MS,
        }
    }
}
