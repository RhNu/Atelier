use thiserror::Error;

pub type SettingsResult<T> = Result<T, SettingsError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SettingsErrorKind {
    InvalidValue,
    Repository,
}

impl std::fmt::Display for SettingsErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidValue => "invalid_value",
            Self::Repository => "repository",
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct SettingsError {
    pub kind: SettingsErrorKind,
    pub message: String,
}

impl SettingsError {
    #[must_use]
    pub fn invalid_value(message: impl Into<String>) -> Self {
        Self {
            kind: SettingsErrorKind::InvalidValue,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self {
            kind: SettingsErrorKind::Repository,
            message: message.into(),
        }
    }
}
