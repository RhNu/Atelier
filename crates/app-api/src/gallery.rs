use serde::{Deserialize, Serialize};

use crate::resource::ResourceRefDto;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyOverrideDto {
    Safe,
    Sensitive,
    Hidden,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySourceKindDto {
    Generation,
    Director,
    Import,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryQueryDto {
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_kind: Option<GallerySourceKindDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_safety_override: Option<GallerySafetyOverrideDto>,
}

impl Default for GalleryQueryDto {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            artifact_kind: None,
            source_kind: None,
            manual_safety_override: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GalleryPageDto {
    pub items: Vec<GalleryItemDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    pub sample_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety: Option<GallerySafetyDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_safety_override: Option<GallerySafetyOverrideDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyScanStateDto {
    Unscanned,
    Scanned,
    Failed,
    Unavailable,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyRiskBandDto {
    Low,
    Medium,
    High,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GallerySafetyLabelDto {
    Safe,
    Sensitive,
    Hidden,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GallerySafetyScoreDto {
    pub label: String,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GallerySafetyDto {
    pub scan_state: GallerySafetyScanStateDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_band: Option<GallerySafetyRiskBandDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_label: Option<GallerySafetyLabelDto>,
    pub effective_label: GallerySafetyLabelDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw_score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_score: Option<f32>,
    pub raw_scores: Vec<GallerySafetyScoreDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scorer_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assessed_at_ms: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualAssetDto {
    pub role: String,
    pub resource: ResourceRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetGallerySafetyOverrideRequestDto {
    pub item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_safety_override: Option<GallerySafetyOverrideDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GalleryImageReferenceTargetDto {
    Director,
    ImageToImage,
    Vibe,
    PreciseReference,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryImageReferenceRequestDto {
    pub item_id: String,
    pub target: GalleryImageReferenceTargetDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryImageReferenceDto {
    pub item_id: String,
    pub artifact_id: String,
    pub target: GalleryImageReferenceTargetDto,
    pub resource: ResourceRefDto,
    pub asset_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_kind: Option<String>,
}
