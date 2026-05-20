use thiserror::Error;

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("database: {message}")]
pub struct DatabaseError {
    message: String,
}

impl DatabaseError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
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
