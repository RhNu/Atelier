use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImageAnalysisModelIdDto {
    AnimeDbRating,
    WdSwinv2TaggerV3,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImageAnalysisModelStateDto {
    Missing,
    Installing,
    Ready,
    Corrupt,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImageAnalysisModelStatusDto {
    pub model_id: ImageAnalysisModelIdDto,
    pub required: bool,
    pub state: ImageAnalysisModelStateDto,
    pub revision: String,
    pub size_bytes: u64,
    pub downloaded_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImageAnalysisModelInstallProgressDto {
    pub model_id: ImageAnalysisModelIdDto,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImageAnalysisModelRequestDto {
    pub model_id: ImageAnalysisModelIdDto,
}
