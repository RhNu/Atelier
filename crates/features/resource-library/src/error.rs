use thiserror::Error;

pub type ResourceLibraryResult<T> = Result<T, ResourceLibraryError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceLibraryErrorKind {
    InvalidIdentifier,
    InvalidPath,
    InvalidName,
    NotFound,
    Conflict,
    Cycle,
    Repository,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct ResourceLibraryError {
    kind: ResourceLibraryErrorKind,
    message: String,
}

impl ResourceLibraryError {
    #[must_use]
    pub fn new(kind: ResourceLibraryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ResourceLibraryErrorKind {
        self.kind
    }
}
