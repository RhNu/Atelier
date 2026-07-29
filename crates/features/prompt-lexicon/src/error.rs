use thiserror::Error;

pub type LexiconResult<T> = Result<T, LexiconError>;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum LexiconError {
    #[error("lexicon unavailable: {0}")]
    Unavailable(String),
    #[error("invalid lexicon request: {0}")]
    InvalidRequest(String),
    #[error("lexicon entity {0} was not found")]
    NotFound(u64),
    #[error("invalid lexicon bundle: {0}")]
    InvalidBundle(String),
    #[error("lexicon query failed: {0}")]
    Query(String),
    #[error("semantic search unavailable: {0}")]
    SemanticUnavailable(String),
}

impl LexiconError {
    #[must_use]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable(message.into())
    }

    #[must_use]
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }

    #[must_use]
    pub fn invalid_bundle(message: impl Into<String>) -> Self {
        Self::InvalidBundle(message.into())
    }

    #[must_use]
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query(message.into())
    }
}
