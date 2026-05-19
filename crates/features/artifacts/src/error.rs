use thiserror::Error;

pub type ArtifactResult<T> = Result<T, ArtifactError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactErrorKind {
    InvalidResourceKind,
    Repository,
    Resource,
}

impl std::fmt::Display for ArtifactErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::InvalidResourceKind => "invalid_resource_kind",
            Self::Repository => "repository",
            Self::Resource => "resource",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct ArtifactError {
    pub kind: ArtifactErrorKind,
    pub message: String,
}

impl ArtifactError {
    #[must_use]
    pub fn invalid_resource_kind(message: impl Into<String>) -> Self {
        Self::new(ArtifactErrorKind::InvalidResourceKind, message)
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self::new(ArtifactErrorKind::Repository, message)
    }

    #[must_use]
    pub fn resource(message: impl Into<String>) -> Self {
        Self::new(ArtifactErrorKind::Resource, message)
    }

    #[must_use]
    pub fn new(kind: ArtifactErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
