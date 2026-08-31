//! `NovelAI` Explore product data; HTTP and undocumented wire fields belong in its adapter.

use crate::{ExploreError, ExploreResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NovelAiExploreSort {
    New,
    Top,
    Hot,
    Random,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NovelAiExplorePeriod {
    Day,
    Week,
    Month,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NovelAiExploreQuery {
    pub tags: Vec<String>,
    pub sort: NovelAiExploreSort,
    pub period: Option<NovelAiExplorePeriod>,
    pub creator_id: Option<String>,
    pub random_salt: Option<String>,
}

impl NovelAiExploreQuery {
    /// Checks combinations before any remote request.
    ///
    /// # Errors
    /// Rejects oversized tags, invalid creator IDs, and incompatible sort options.
    pub fn validate(&self) -> ExploreResult<()> {
        if self.tags.len() > 20
            || self
                .tags
                .iter()
                .any(|tag| tag.trim().is_empty() || tag.len() > 256)
            || self.tags.iter().map(String::len).sum::<usize>() > 2_048
        {
            return Err(ExploreError::invalid("invalid or oversized Explore tags"));
        }
        if self.creator_id.as_ref().is_some_and(|id| {
            id.is_empty()
                || id.len() > 128
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        }) {
            return Err(ExploreError::invalid("invalid Explore creator ID"));
        }
        if (self.sort == NovelAiExploreSort::New) != self.period.is_none() {
            return Err(ExploreError::invalid(
                "New has no period; other sorts require a period",
            ));
        }
        if self.sort == NovelAiExploreSort::Random {
            if !self.tags.is_empty() || self.creator_id.is_some() {
                return Err(ExploreError::invalid(
                    "Random cannot be combined with tags or a creator",
                ));
            }
            if self.random_salt.as_ref().is_none_or(|salt| {
                salt.len() != 6 || !salt.bytes().all(|b| b.is_ascii_alphanumeric())
            }) {
                return Err(ExploreError::invalid(
                    "Random requires a six-character alphanumeric salt",
                ));
            }
        } else if self.random_salt.is_some() {
            return Err(ExploreError::invalid(
                "random salt is only valid for Random",
            ));
        }
        Ok(())
    }
}

/// Prevent path/query injection without accepting arbitrary URLs.
///
/// # Errors
/// Rejects identifiers other than canonical UUID-shaped post IDs.
pub fn validate_post_id(id: &str) -> ExploreResult<()> {
    if id.len() != 36
        || !id.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
    {
        return Err(ExploreError::invalid("invalid NovelAI Explore post ID"));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct NovelAiExplorePost {
    pub id: String,
    pub title: String,
    pub description: String,
    pub creator_id: Option<String>,
    pub creator_name: Option<String>,
    pub created_at: String,
    pub width: u32,
    pub height: u32,
    pub like_count: Option<u64>,
    pub metadata: NovelAiExploreMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExploreMetadataStatus {
    Available,
    Missing,
    Partial,
    Invalid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExploreCharacterCaption {
    pub text: String,
    pub centers: Vec<ExploreCharacterCenter>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExploreCharacterCenter {
    pub x: f64,
    pub y: f64,
}

/// Positive and negative character arrays remain separate to avoid inventing pairings.
#[derive(Clone, Debug, PartialEq)]
pub struct NovelAiExploreMetadata {
    pub status: ExploreMetadataStatus,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub characters: Vec<ExploreCharacterCaption>,
    pub negative_characters: Vec<ExploreCharacterCaption>,
    pub use_coords: Option<bool>,
    pub use_order: Option<bool>,
    pub negative_use_coords: Option<bool>,
    pub negative_use_order: Option<bool>,
    pub parameters: Vec<ExploreGenerationParameter>,
    pub raw: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExploreGenerationParameter {
    pub name: String,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random() -> NovelAiExploreQuery {
        NovelAiExploreQuery {
            tags: vec![],
            sort: NovelAiExploreSort::Random,
            period: Some(NovelAiExplorePeriod::Week),
            creator_id: None,
            random_salt: Some("abc123".into()),
        }
    }

    #[test]
    fn random_requires_an_isolated_session() {
        assert!(random().validate().is_ok());
        let mut query = random();
        query.tags.push("solo".into());
        assert!(query.validate().is_err());
        query.tags.clear();
        query.creator_id = Some("creator".into());
        assert!(query.validate().is_err());
        query.creator_id = None;
        query.random_salt = None;
        assert!(query.validate().is_err());
    }

    #[test]
    fn new_and_period_sorts_have_distinct_options() {
        let mut query = random();
        query.sort = NovelAiExploreSort::New;
        assert!(query.validate().is_err());
        query.period = None;
        query.random_salt = None;
        assert!(query.validate().is_ok());
        query.sort = NovelAiExploreSort::Hot;
        assert!(query.validate().is_err());
    }

    #[test]
    fn post_ids_cannot_escape_the_fixed_endpoint() {
        assert!(validate_post_id("00000000-0000-0000-0000-000000000001").is_ok());
        for id in ["../search", "https://example.com", "id?token=x", ""] {
            assert!(validate_post_id(id).is_err());
        }
    }
}
