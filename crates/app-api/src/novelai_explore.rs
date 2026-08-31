use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum NovelAiExploreSortDto {
    New,
    Top,
    Hot,
    Random,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum NovelAiExplorePeriodDto {
    Day,
    Week,
    Month,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct NovelAiExploreQueryDto {
    pub tags: Vec<String>,
    pub sort: NovelAiExploreSortDto,
    pub period: Option<NovelAiExplorePeriodDto>,
    pub creator_id: Option<String>,
    pub random_salt: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct NovelAiExplorePostSummaryDto {
    pub id: String,
    pub title: String,
    pub creator_id: Option<String>,
    pub creator_name: Option<String>,
    pub width: u32,
    pub height: u32,
    pub like_count: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct NovelAiExplorePostDetailDto {
    pub post: NovelAiExplorePostSummaryDto,
    pub description: String,
    pub created_at: String,
    pub page_url: String,
    pub metadata: NovelAiExploreMetadataDto,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ExploreMetadataStatusDto {
    Available,
    Missing,
    Partial,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ExploreCharacterCaptionDto {
    pub text: String,
    pub centers: Vec<ExploreCharacterCenterDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ExploreCharacterCenterDto {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExploreGenerationParameterDto {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct NovelAiExploreMetadataDto {
    pub status: ExploreMetadataStatusDto,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub characters: Vec<ExploreCharacterCaptionDto>,
    pub negative_characters: Vec<ExploreCharacterCaptionDto>,
    pub use_coords: Option<bool>,
    pub use_order: Option<bool>,
    pub negative_use_coords: Option<bool>,
    pub negative_use_order: Option<bool>,
    pub parameters: Vec<ExploreGenerationParameterDto>,
    pub raw: Option<String>,
    pub warnings: Vec<String>,
}
