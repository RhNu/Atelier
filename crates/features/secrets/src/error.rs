use nai_atelier_foundation::NovelAiError;
use thiserror::Error;

pub type SecretsResult<T> = Result<T, SecretsError>;
pub type ProbeApiKeyResult<T> = Result<T, ProbeApiKeyError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretsErrorKind {
    Validation,
    MetadataStore,
    SecretStore,
    MissingActiveKey,
    MissingSecret,
}

impl std::fmt::Display for SecretsErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Validation => "validation",
            Self::MetadataStore => "metadata_store",
            Self::SecretStore => "secret_store",
            Self::MissingActiveKey => "missing_active_key",
            Self::MissingSecret => "missing_secret",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct SecretsError {
    pub kind: SecretsErrorKind,
    pub message: String,
}

impl SecretsError {
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(SecretsErrorKind::Validation, message)
    }

    #[must_use]
    pub fn metadata_store(message: impl Into<String>) -> Self {
        Self::new(SecretsErrorKind::MetadataStore, message)
    }

    #[must_use]
    pub fn secret_store(message: impl Into<String>) -> Self {
        Self::new(SecretsErrorKind::SecretStore, message)
    }

    #[must_use]
    pub fn missing_active_key() -> Self {
        Self::new(SecretsErrorKind::MissingActiveKey, "no active API key")
    }

    #[must_use]
    pub fn missing_secret(id: impl std::fmt::Display) -> Self {
        Self::new(
            SecretsErrorKind::MissingSecret,
            format!("missing secret `{id}`"),
        )
    }

    #[must_use]
    pub fn new(kind: SecretsErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ProbeApiKeyError {
    #[error("secret resolution failed: {0}")]
    Secrets(#[from] SecretsError),
    #[error("subscription probe failed: {0}")]
    NovelAi(#[from] NovelAiError),
}
