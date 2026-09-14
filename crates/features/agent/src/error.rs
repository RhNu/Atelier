use thiserror::Error;

pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentErrorKind {
    Validation,
    NotFound,
    Conflict,
    Repository,
}

impl std::fmt::Display for AgentErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Validation => "validation",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::Repository => "repository",
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct AgentError {
    pub kind: AgentErrorKind,
    pub message: String,
}

impl AgentError {
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::Validation, message)
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::NotFound, message)
    }

    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::Conflict, message)
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::Repository, message)
    }

    #[must_use]
    pub fn new(kind: AgentErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
