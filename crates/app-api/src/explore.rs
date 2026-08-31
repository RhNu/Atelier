use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    danbooru::{DanbooruPostDetailDto, DanbooruPostSummaryDto, DanbooruRatingDto},
    novelai_explore::{
        NovelAiExplorePostDetailDto, NovelAiExplorePostSummaryDto, NovelAiExploreQueryDto,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExploreSourceIdDto {
    DanbooruDatabase,
    NovelaiExploreGallery,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExploreSourceDescriptorDto {
    pub id: ExploreSourceIdDto,
    pub name: String,
    pub experimental: bool,
    pub supports_account: bool,
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExploreItemRefDto {
    pub source_id: ExploreSourceIdDto,
    pub item_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DanbooruExploreQueryDto {
    pub query: String,
    pub ratings: Vec<DanbooruRatingDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "source_id", content = "query", rename_all = "snake_case")]
pub enum ExploreQueryDto {
    DanbooruDatabase(DanbooruExploreQueryDto),
    NovelaiExploreGallery(NovelAiExploreQueryDto),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExploreSearchRequestDto {
    pub query: ExploreQueryDto,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "source_id", content = "post", rename_all = "snake_case")]
pub enum ExplorePostSummaryDto {
    DanbooruDatabase(DanbooruPostSummaryDto),
    NovelaiExploreGallery(NovelAiExplorePostSummaryDto),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExplorePageDto {
    pub items: Vec<ExplorePostSummaryDto>,
    pub next_cursor: Option<String>,
    pub total: Option<u64>,
    pub authenticated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "source_id", content = "detail", rename_all = "snake_case")]
pub enum ExplorePostDetailDto {
    DanbooruDatabase(DanbooruPostDetailDto),
    NovelaiExploreGallery(NovelAiExplorePostDetailDto),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExploreMediaVariantDto {
    Thumbnail,
    Preview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExploreMediaRequestDto {
    pub item: ExploreItemRefDto,
    pub variant: ExploreMediaVariantDto,
}
