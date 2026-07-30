//! Danbooru discovery domain contracts.

use async_trait::async_trait;
use atelier_secrets::SecretValue;
use std::collections::HashSet;
use thiserror::Error;

pub const DANBOORU_PAGE_SIZE: usize = 40;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DanbooruRating {
    General,
    Sensitive,
    Questionable,
    Explicit,
}

impl DanbooruRating {
    #[must_use]
    pub const fn query_code(self) -> char {
        match self {
            Self::General => 'g',
            Self::Sensitive => 's',
            Self::Questionable => 'q',
            Self::Explicit => 'e',
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DanbooruTagCategory {
    Artist,
    Copyright,
    Character,
    General,
    Meta,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DanbooruMediaVariant {
    Preview,
    Sample,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanbooruSearchRequest {
    pub query: String,
    pub ratings: Vec<DanbooruRating>,
    pub before_id: Option<u64>,
}

impl DanbooruSearchRequest {
    /// Validates the user query and constructs the authoritative provider query.
    ///
    /// # Errors
    /// Returns an error for empty rating filters, oversized queries, or rating
    /// metatags that conflict with the product-level safety controls.
    pub fn provider_tags(&self) -> DanbooruResult<String> {
        if self.query.chars().count() > 1_024 {
            return Err(DanbooruError::invalid_request(
                "query must not exceed 1024 characters",
            ));
        }
        if self.ratings.is_empty() {
            return Err(DanbooruError::invalid_request(
                "at least one rating must be selected",
            ));
        }
        for token in self.query.split_whitespace() {
            let normalized = token.trim_start_matches('-').to_ascii_lowercase();
            if normalized.starts_with("rating:")
                || matches!(
                    normalized.as_str(),
                    "is:sfw"
                        | "is:nsfw"
                        | "is:general"
                        | "is:sensitive"
                        | "is:questionable"
                        | "is:explicit"
                )
            {
                return Err(DanbooruError::invalid_request(
                    "rating metatags must be controlled with the rating filters",
                ));
            }
        }

        let mut seen = HashSet::new();
        let mut rating_codes = String::new();
        for rating in &self.ratings {
            if seen.insert(*rating) {
                if !rating_codes.is_empty() {
                    rating_codes.push(',');
                }
                rating_codes.push(rating.query_code());
            }
        }
        let query = self.query.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(if query.is_empty() {
            format!("rating:{rating_codes}")
        } else {
            format!("{query} rating:{rating_codes}")
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanbooruPost {
    pub id: u64,
    pub created_at: String,
    pub rating: DanbooruRating,
    pub width: u32,
    pub height: u32,
    pub score: i64,
    pub favorite_count: u64,
    pub file_extension: String,
    pub file_size: u64,
    pub source_url: Option<String>,
    pub preview_url: Option<String>,
    pub sample_url: Option<String>,
    pub artist_tags: Vec<String>,
    pub copyright_tags: Vec<String>,
    pub character_tags: Vec<String>,
    pub general_tags: Vec<String>,
    pub meta_tags: Vec<String>,
}

impl DanbooruPost {
    #[must_use]
    pub fn media_url(&self, variant: DanbooruMediaVariant) -> Option<&str> {
        match variant {
            DanbooruMediaVariant::Preview => self.preview_url.as_deref(),
            DanbooruMediaVariant::Sample => self.sample_url.as_deref(),
        }
    }

    #[must_use]
    pub fn ordered_tags(&self) -> Vec<(DanbooruTagCategory, &str)> {
        let groups = [
            (DanbooruTagCategory::Artist, &self.artist_tags),
            (DanbooruTagCategory::Copyright, &self.copyright_tags),
            (DanbooruTagCategory::Character, &self.character_tags),
            (DanbooruTagCategory::General, &self.general_tags),
            (DanbooruTagCategory::Meta, &self.meta_tags),
        ];
        groups
            .into_iter()
            .flat_map(|(category, tags)| tags.iter().map(move |tag| (category, tag.as_str())))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanbooruPostPage {
    pub posts: Vec<DanbooruPost>,
    pub next_before_id: Option<u64>,
    pub authenticated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanbooruMedia {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanbooruProfile {
    pub username: String,
    pub level: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct DanbooruCredentials {
    pub username: String,
    pub api_key: SecretValue,
}

impl std::fmt::Debug for DanbooruCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DanbooruCredentials")
            .field("username", &self.username)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DanbooruErrorKind {
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
pub struct DanbooruError {
    pub kind: DanbooruErrorKind,
    pub message: String,
    pub retry_after_seconds: Option<u64>,
}

impl DanbooruError {
    #[must_use]
    pub fn new(kind: DanbooruErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retry_after_seconds: None,
        }
    }

    #[must_use]
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(DanbooruErrorKind::InvalidRequest, message)
    }

    #[must_use]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(DanbooruErrorKind::Unavailable, message)
    }

    #[must_use]
    pub const fn with_retry_after(mut self, seconds: Option<u64>) -> Self {
        self.retry_after_seconds = seconds;
        self
    }
}

pub type DanbooruResult<T> = Result<T, DanbooruError>;

#[async_trait]
pub trait DanbooruClient: Send + Sync {
    async fn search(
        &self,
        request: DanbooruSearchRequest,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPostPage>;

    async fn post(
        &self,
        post_id: u64,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPost>;

    async fn media(
        &self,
        post_id: u64,
        variant: DanbooruMediaVariant,
        credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruMedia>;

    async fn profile(&self, credentials: &DanbooruCredentials) -> DanbooruResult<DanbooruProfile>;
}

#[derive(Debug, Default)]
pub struct UnavailableDanbooruClient;

#[async_trait]
impl DanbooruClient for UnavailableDanbooruClient {
    async fn search(
        &self,
        _request: DanbooruSearchRequest,
        _credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPostPage> {
        Err(DanbooruError::unavailable("Danbooru client is unavailable"))
    }

    async fn post(
        &self,
        _post_id: u64,
        _credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruPost> {
        Err(DanbooruError::unavailable("Danbooru client is unavailable"))
    }

    async fn media(
        &self,
        _post_id: u64,
        _variant: DanbooruMediaVariant,
        _credentials: Option<&DanbooruCredentials>,
    ) -> DanbooruResult<DanbooruMedia> {
        Err(DanbooruError::unavailable("Danbooru client is unavailable"))
    }

    async fn profile(&self, _credentials: &DanbooruCredentials) -> DanbooruResult<DanbooruProfile> {
        Err(DanbooruError::unavailable("Danbooru client is unavailable"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_query_normalizes_whitespace_and_appends_ratings() {
        let request = DanbooruSearchRequest {
            query: "  1girl   order:score ".to_owned(),
            ratings: vec![DanbooruRating::General, DanbooruRating::Sensitive],
            before_id: None,
        };
        assert_eq!(
            request.provider_tags().unwrap(),
            "1girl order:score rating:g,s"
        );
    }

    #[test]
    fn provider_query_rejects_rating_metatags() {
        for query in ["rating:e", "-rating:s", "is:nsfw"] {
            let request = DanbooruSearchRequest {
                query: query.to_owned(),
                ratings: vec![DanbooruRating::General],
                before_id: None,
            };
            assert_eq!(
                request.provider_tags().unwrap_err().kind,
                DanbooruErrorKind::InvalidRequest
            );
        }
    }
}
