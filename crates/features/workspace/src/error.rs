use thiserror::Error;

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceErrorKind {
    InvalidPath,
    Locked,
    Storage,
    UnsupportedSchema,
}

impl std::fmt::Display for WorkspaceErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::InvalidPath => "invalid_path",
            Self::Locked => "locked",
            Self::Storage => "storage",
            Self::UnsupportedSchema => "unsupported_schema",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct WorkspaceError {
    pub kind: WorkspaceErrorKind,
    pub message: String,
}

impl WorkspaceError {
    #[must_use]
    pub fn new(kind: WorkspaceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::new(
            WorkspaceErrorKind::InvalidPath,
            format!("invalid workspace path: {}", message.into()),
        )
    }

    #[must_use]
    pub fn locked(message: impl Into<String>) -> Self {
        Self::new(WorkspaceErrorKind::Locked, message)
    }

    #[must_use]
    pub fn storage(message: impl Into<String>) -> Self {
        Self::new(WorkspaceErrorKind::Storage, message)
    }

    #[must_use]
    pub fn unsupported_schema(message: impl Into<String>) -> Self {
        Self::new(WorkspaceErrorKind::UnsupportedSchema, message)
    }
}
