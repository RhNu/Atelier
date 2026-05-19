//! Shared foundation crate for NAI Atelier.
//!
//! Keep this crate intentionally thin. Promote APIs here only after more than
//! one feature needs the same stable primitive or contract.

use std::time::Duration;

use thiserror::Error;

/// Cross-feature error category for NovelAI-facing ports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NovelAiErrorKind {
    InvalidRequest,
    Authentication,
    InsufficientCredit,
    RequestConflict,
    RateLimited,
    ServiceUnavailable,
    Transport,
    Decode,
    Metadata,
    UnknownApi,
}

impl std::fmt::Display for NovelAiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::InvalidRequest => "invalid_request",
            Self::Authentication => "authentication",
            Self::InsufficientCredit => "insufficient_credit",
            Self::RequestConflict => "request_conflict",
            Self::RateLimited => "rate_limited",
            Self::ServiceUnavailable => "service_unavailable",
            Self::Transport => "transport",
            Self::Decode => "decode",
            Self::Metadata => "metadata",
            Self::UnknownApi => "unknown_api",
        };
        f.write_str(value)
    }
}

/// Stable error shape returned by internal `NovelAI` ports.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct NovelAiError {
    pub kind: NovelAiErrorKind,
    pub message: String,
    pub status: Option<u16>,
    pub retry_after: Option<Duration>,
}

impl NovelAiError {
    #[must_use]
    pub fn new(kind: NovelAiErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            status: None,
            retry_after: None,
        }
    }

    #[must_use]
    pub const fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    #[must_use]
    pub const fn with_retry_after(mut self, retry_after: Duration) -> Self {
        self.retry_after = Some(retry_after);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-foundation");
    }

    #[test]
    fn novelai_error_keeps_category_and_retry_after() {
        let error = NovelAiError::new(NovelAiErrorKind::RateLimited, "slow down")
            .with_status(429)
            .with_retry_after(Duration::from_secs(2));

        assert_eq!(error.kind, NovelAiErrorKind::RateLimited);
        assert_eq!(error.status, Some(429));
        assert_eq!(error.retry_after, Some(Duration::from_secs(2)));
    }
}
