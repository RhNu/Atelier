use thiserror::Error;

pub type JobResult<T> = Result<T, JobQueueError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JobQueueErrorKind {
    Conflict,
    EmptyBatch,
    InvalidState,
    NotFound,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {message}")]
pub struct JobQueueError {
    kind: JobQueueErrorKind,
    message: String,
}

impl JobQueueError {
    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(JobQueueErrorKind::Conflict, message)
    }

    #[must_use]
    pub fn empty_batch(message: impl Into<String>) -> Self {
        Self::new(JobQueueErrorKind::EmptyBatch, message)
    }

    #[must_use]
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(JobQueueErrorKind::InvalidState, message)
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(JobQueueErrorKind::NotFound, message)
    }

    #[must_use]
    pub const fn kind(&self) -> JobQueueErrorKind {
        self.kind
    }

    fn new(kind: JobQueueErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
