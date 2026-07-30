use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DanbooruAccountStateDto {
    Anonymous,
    Configured,
    Verified,
    Invalid,
    KeyringUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruAccountDto {
    pub configured: bool,
    pub state: DanbooruAccountStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SaveDanbooruAccountRequestDto {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DanbooruRatingDto {
    General,
    Sensitive,
    Questionable,
    Explicit,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DanbooruAuthModeDto {
    Anonymous,
    Authenticated,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DanbooruTagCategoryDto {
    Artist,
    Copyright,
    Character,
    General,
    Meta,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum DanbooruMediaVariantDto {
    Preview,
    Sample,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruSearchRequestDto {
    pub query: String,
    pub ratings: Vec<DanbooruRatingDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruPostSummaryDto {
    pub id: u64,
    pub rating: DanbooruRatingDto,
    pub width: u32,
    pub height: u32,
    pub score: i64,
    pub favorite_count: u64,
    pub file_extension: String,
    pub tag_count: usize,
    pub has_preview: bool,
    pub has_sample: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruPostPageDto {
    pub items: Vec<DanbooruPostSummaryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_before_id: Option<u64>,
    pub auth_mode: DanbooruAuthModeDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruPostDetailRequestDto {
    pub post_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruTagDto {
    pub canonical_name: String,
    pub category: DanbooruTagCategoryDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_count: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruPostDetailDto {
    pub post: DanbooruPostSummaryDto,
    pub created_at: String,
    pub file_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub danbooru_url: String,
    pub tags: Vec<DanbooruTagDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruMediaRequestDto {
    pub post_id: u64,
    pub variant: DanbooruMediaVariantDto,
}
