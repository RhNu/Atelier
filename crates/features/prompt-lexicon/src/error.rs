use super::Error;

#[derive(Debug, Error)]
pub enum PromptLexiconError {
    #[error("failed to parse prompt lexicon: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported prompt lexicon schema `{schema}` version {version}")]
    UnsupportedSchema { schema: String, version: u32 },
    #[error("invalid prompt lexicon payload: {0}")]
    InvalidPayload(String),
    #[error("invalid prompt lexicon request: {0}")]
    InvalidRequest(String),
}
