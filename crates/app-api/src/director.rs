use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::gallery::GalleryItemDto;
use crate::resource::{ImageInputDto, ResourceRefDto};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DirectorToolDto {
    #[default]
    Lineart,
    Sketch,
    BgRemoval,
    Emotion,
    Declutter,
    Colorize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunDirectorToolRequestDto {
    pub run_id: String,
    pub tool: DirectorToolDto,
    pub image: ImageInputDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defry: Option<u8>,
    pub strict_mode: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct DirectorToolResultDto {
    pub item_id: String,
    pub artifact_id: String,
    pub resource: ResourceRefDto,
    pub item: GalleryItemDto,
}
