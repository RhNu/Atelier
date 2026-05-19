use thiserror::Error;

pub type SafetyResult<T> = Result<T, SafetyError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SafetyErrorKind {
    InvalidScore,
    Scanner,
}

impl std::fmt::Display for SafetyErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::InvalidScore => "invalid_score",
            Self::Scanner => "scanner",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct SafetyError {
    kind: SafetyErrorKind,
    message: String,
}

impl SafetyError {
    #[must_use]
    pub fn invalid_score(message: impl Into<String>) -> Self {
        Self::new(SafetyErrorKind::InvalidScore, message)
    }

    #[must_use]
    pub fn scanner(message: impl Into<String>) -> Self {
        Self::new(SafetyErrorKind::Scanner, message)
    }

    #[must_use]
    pub const fn kind(&self) -> SafetyErrorKind {
        self.kind
    }

    fn new(kind: SafetyErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
