use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::resource::ResourceRefDto;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyOverrideDto {
    Safe,
    Sensitive,
    Hidden,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GallerySourceKindDto {
    Generation,
    Director,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GalleryQueryDto {
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<GallerySourceKindDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_safety_override: Option<GallerySafetyOverrideDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_label: Option<GallerySafetyLabelDto>,
}

impl Default for GalleryQueryDto {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            artifact_kind: None,
            source_kind: None,
            manual_safety_override: None,
            safety_label: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GalleryPageDto {
    pub items: Vec<GalleryItemDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GalleryItemDto {
    pub item_id: String,
    pub artifact_id: String,
    pub artifact_kind: String,
    pub source_kind: GallerySourceKindDto,
    pub primary_resource: ResourceRefDto,
    pub assets: Vec<VisualAssetDto>,
    pub indexed_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_metadata_status: Option<GalleryMetadataStatusDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_metadata_error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embedded_metadata_warnings: Vec<GalleryMetadataWarningDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    pub safety: GallerySafetyDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_safety_override: Option<GallerySafetyOverrideDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GalleryMetadataStatusDto {
    Parsed,
    NotPresent,
    UnsupportedFormat,
    Invalid,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GalleryMetadataWarningCodeDto {
    InvalidCommentJson,
    InvalidTextChunk,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GalleryMetadataWarningDto {
    pub code: GalleryMetadataWarningCodeDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GalleryItemDetailRequestDto {
    pub item_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GalleryItemDetailDto {
    pub item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_metadata_json: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyScanStateDto {
    Unscanned,
    Scanned,
    Failed,
    Unavailable,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyRiskBandDto {
    Low,
    Medium,
    High,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyLabelDto {
    Safe,
    Sensitive,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GallerySafetyRatingScoresDto {
    pub general: f32,
    pub sensitive: f32,
    pub questionable: f32,
    pub explicit: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GallerySafetyModelEvidenceDto {
    pub model_id: String,
    pub model_revision: String,
    pub ratings: GallerySafetyRatingScoresDto,
    pub fused_score: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyReviewStateDto {
    NotNeeded,
    Disabled,
    Completed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GallerySafetyReviewDto {
    pub state: GallerySafetyReviewStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<GallerySafetyModelEvidenceDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GallerySafetyDto {
    pub scan_state: GallerySafetyScanStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_band: Option<GallerySafetyRiskBandDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_label: Option<GallerySafetyLabelDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_label: Option<GallerySafetyLabelDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<GallerySafetyModelEvidenceDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<GallerySafetyReviewDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessed_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct VisualAssetDto {
    pub role: String,
    pub resource: ResourceRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SetGallerySafetyOverrideRequestDto {
    pub item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_safety_override: Option<GallerySafetyOverrideDto>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RescanGallerySafetyRequestDto {
    #[serde(default)]
    pub item_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RescanGallerySafetyResponseDto {
    pub requested: usize,
    pub scanned: usize,
    pub failed: usize,
    pub unavailable: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteGalleryItemsRequestDto {
    pub item_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteGalleryItemsResponseDto {
    pub deleted: usize,
    pub resources_released: usize,
    pub blobs_deleted: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GalleryImageReferenceTargetDto {
    Director,
    ImageToImage,
    Vibe,
    PreciseReference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GalleryImageReferenceRequestDto {
    pub item_id: String,
    pub target: GalleryImageReferenceTargetDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GalleryImageReferenceDto {
    pub item_id: String,
    pub artifact_id: String,
    pub target: GalleryImageReferenceTargetDto,
    pub resource: ResourceRefDto,
    pub asset_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_kind: Option<String>,
}
