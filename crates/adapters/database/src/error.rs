use thiserror::Error;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DatabaseErrorKind {
    Database,
    UnsupportedSchema,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("database: {message}")]
pub struct DatabaseError {
    kind: DatabaseErrorKind,
    message: String,
}

impl DatabaseError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            kind: DatabaseErrorKind::Database,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn unsupported_schema(message: impl Into<String>) -> Self {
        Self {
            kind: DatabaseErrorKind::UnsupportedSchema,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> DatabaseErrorKind {
        self.kind
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for DatabaseError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}
