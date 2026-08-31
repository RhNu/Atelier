//! Shared entry contracts for remote, read-only discovery sources.

pub mod novelai;

use async_trait::async_trait;
use atelier_danbooru::{DanbooruError, DanbooruErrorKind, DanbooruRating};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanbooruExploreQuery {
    pub query: String,
    pub ratings: Vec<DanbooruRating>,
}

/// Continuations stay on the backend; app-api exposes query-bound opaque tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreCursor {
    BeforeId(u64),
    Offset(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplorePage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<ExploreCursor>,
    pub total: Option<u64>,
    pub authenticated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreMediaVariant {
    Thumbnail,
    Preview,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExploreMedia {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreErrorKind {
    InvalidRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimited,
    Unavailable,
    InvalidResponse,
    MediaRejected,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct ExploreError {
    pub kind: ExploreErrorKind,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

impl ExploreError {
    #[must_use]
    pub fn new(kind: ExploreErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after_seconds: None,
        }
    }

    #[must_use]
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ExploreErrorKind::InvalidRequest, message)
    }
}

impl From<DanbooruError> for ExploreError {
    fn from(error: DanbooruError) -> Self {
        let kind = match error.kind {
            DanbooruErrorKind::InvalidRequest => ExploreErrorKind::InvalidRequest,
            DanbooruErrorKind::Unauthorized => ExploreErrorKind::Unauthorized,
            DanbooruErrorKind::Forbidden => ExploreErrorKind::Forbidden,
            DanbooruErrorKind::NotFound => ExploreErrorKind::NotFound,
            DanbooruErrorKind::RateLimited => ExploreErrorKind::RateLimited,
            DanbooruErrorKind::Unavailable => ExploreErrorKind::Unavailable,
            DanbooruErrorKind::InvalidResponse => ExploreErrorKind::InvalidResponse,
            DanbooruErrorKind::MediaRejected => ExploreErrorKind::MediaRejected,
        };
        Self {
            kind,
            message: error.message,
            retry_after_seconds: error.retry_after_seconds,
        }
    }
}

pub type ExploreResult<T> = Result<T, ExploreError>;

/// Sources share operations, not their query language or post schema.
#[async_trait]
pub trait ExploreSource: Send + Sync {
    type Query: Send;
    type Post: Send;

    async fn search(
        &self,
        query: Self::Query,
        cursor: Option<ExploreCursor>,
    ) -> ExploreResult<ExplorePage<Self::Post>>;
    async fn detail(&self, item_id: &str) -> ExploreResult<Self::Post>;
    async fn media(
        &self,
        item_id: &str,
        variant: ExploreMediaVariant,
    ) -> ExploreResult<ExploreMedia>;
}
