use thiserror::Error;

pub type GalleryResult<T> = Result<T, GalleryError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GalleryErrorKind {
    NotFound,
    Repository,
}

impl std::fmt::Display for GalleryErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::NotFound => "not_found",
            Self::Repository => "repository",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct GalleryError {
    kind: GalleryErrorKind,
    message: String,
}

impl GalleryError {
    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(GalleryErrorKind::NotFound, message)
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self::new(GalleryErrorKind::Repository, message)
    }

    #[must_use]
    pub const fn kind(&self) -> GalleryErrorKind {
        self.kind
    }

    #[must_use]
    pub fn new(kind: GalleryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
