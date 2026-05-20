use thiserror::Error;

pub type VibeDomainResult<T> = Result<T, VibeError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VibeErrorKind {
    InvalidDocument,
    InvalidSettings,
    NotFound,
    Repository,
}

impl VibeErrorKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidDocument => "invalid_document",
            Self::InvalidSettings => "invalid_settings",
            Self::NotFound => "not_found",
            Self::Repository => "repository",
        }
    }
}

impl std::fmt::Display for VibeErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct VibeError {
    kind: VibeErrorKind,
    message: String,
}

impl VibeError {
    #[must_use]
    pub fn new(kind: VibeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn invalid_document(message: impl Into<String>) -> Self {
        Self::new(VibeErrorKind::InvalidDocument, message)
    }

    #[must_use]
    pub fn invalid_settings(message: impl Into<String>) -> Self {
        Self::new(VibeErrorKind::InvalidSettings, message)
    }

    #[must_use]
    pub const fn kind(&self) -> VibeErrorKind {
        self.kind
    }
}
