use super::{
    ArtifactId, ArtifactMetadataDto, ArtifactSourceDto, DatabaseError, DatabaseResult, Deserialize,
    GalleryItem, GalleryItemId, ImageSafetyScore, JSON_SCHEMA_VERSION, JsonCodec, ResourceRefDto,
    SafetyAssessment, SafetyModelScore, Serialize, VisualAssetRefDto, VisualAssetRole,
    artifact_kind_as_str, artifact_kind_from_str, decode_error, ensure_schema,
    safety_override_as_str, safety_override_from_str,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SafetyAssessmentDto {
    resource: ResourceRefDto,
    score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    safe_score: Option<f32>,
    #[serde(default)]
    raw_scores: Vec<SafetyModelScoreDto>,
    scorer_label: Option<String>,
    scorer_version: Option<String>,
    assessed_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SafetyModelScoreDto {
    label: String,
    score: f32,
}

impl From<&SafetyAssessment> for SafetyAssessmentDto {
    fn from(value: &SafetyAssessment) -> Self {
        Self {
            resource: ResourceRefDto::from(&value.resource),
            score: value.score.value(),
            safe_score: value.safe_score.map(ImageSafetyScore::value),
            raw_scores: value
                .raw_scores
                .iter()
                .map(SafetyModelScoreDto::from)
                .collect(),
            scorer_label: value.scorer_label.clone(),
            scorer_version: value.scorer_version.clone(),
            assessed_at_ms: value.assessed_at_ms,
        }
    }
}

impl From<&SafetyModelScore> for SafetyModelScoreDto {
    fn from(value: &SafetyModelScore) -> Self {
        Self {
            label: value.label.clone(),
            score: value.score.value(),
        }
    }
}

impl SafetyModelScoreDto {
    fn into_domain(self) -> DatabaseResult<SafetyModelScore> {
        SafetyModelScore::new(self.label, self.score)
            .map_err(|error| DatabaseError::new(error.to_string()))
    }
}

impl SafetyAssessmentDto {
    fn into_domain(self) -> DatabaseResult<SafetyAssessment> {
        Ok(SafetyAssessment {
            resource: self.resource.into_domain(),
            score: ImageSafetyScore::new(self.score)
                .map_err(|error| DatabaseError::new(error.to_string()))?,
            safe_score: self
                .safe_score
                .map(ImageSafetyScore::new)
                .transpose()
                .map_err(|error| DatabaseError::new(error.to_string()))?,
            raw_scores: self
                .raw_scores
                .into_iter()
                .map(SafetyModelScoreDto::into_domain)
                .collect::<DatabaseResult<Vec<_>>>()?,
            scorer_label: self.scorer_label,
            scorer_version: self.scorer_version,
            assessed_at_ms: self.assessed_at_ms,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GalleryItemDto {
    schema_version: u32,
    id: String,
    artifact_id: String,
    artifact_kind: String,
    source: ArtifactSourceDto,
    primary_resource: ResourceRefDto,
    assets: Vec<VisualAssetRefDto>,
    metadata: ArtifactMetadataDto,
    safety_assessment: Option<SafetyAssessmentDto>,
    manual_safety_override: Option<String>,
    indexed_at_ms: u64,
}

impl JsonCodec<GalleryItem> for GalleryItemDto {
    fn from_domain(value: &GalleryItem) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            id: value.id.as_str().to_owned(),
            artifact_id: value.artifact_id.as_str().to_owned(),
            artifact_kind: artifact_kind_as_str(value.artifact_kind).to_owned(),
            source: ArtifactSourceDto::from(&value.source),
            primary_resource: ResourceRefDto::from(&value.primary_resource),
            assets: value.assets.iter().map(VisualAssetRefDto::from).collect(),
            metadata: ArtifactMetadataDto::from(&value.metadata),
            safety_assessment: value
                .safety_assessment
                .as_ref()
                .map(SafetyAssessmentDto::from),
            manual_safety_override: value
                .manual_safety_override
                .map(safety_override_as_str)
                .map(str::to_owned),
            indexed_at_ms: value.indexed_at_ms,
        }
    }

    fn into_domain(self) -> DatabaseResult<GalleryItem> {
        ensure_schema(self.schema_version)?;
        Ok(GalleryItem {
            id: GalleryItemId::new(self.id),
            artifact_id: ArtifactId::new(self.artifact_id),
            artifact_kind: artifact_kind_from_str(&self.artifact_kind)?,
            source: self.source.into_domain()?,
            primary_resource: self.primary_resource.into_domain(),
            assets: self
                .assets
                .into_iter()
                .map(VisualAssetRefDto::into_domain)
                .collect::<DatabaseResult<Vec<_>>>()?,
            metadata: self.metadata.into(),
            safety_assessment: self
                .safety_assessment
                .map(SafetyAssessmentDto::into_domain)
                .transpose()?,
            manual_safety_override: self
                .manual_safety_override
                .as_deref()
                .map(safety_override_from_str)
                .transpose()?,
            indexed_at_ms: self.indexed_at_ms,
        })
    }
}

pub(super) const fn visual_asset_role_as_str(value: VisualAssetRole) -> &'static str {
    match value {
        VisualAssetRole::Original => "original",
        VisualAssetRole::Thumbnail => "thumbnail",
        VisualAssetRole::Preview => "preview",
        VisualAssetRole::Sanitized => "sanitized",
        VisualAssetRole::Export => "export",
    }
}

pub(super) fn visual_asset_role_from_str(value: &str) -> DatabaseResult<VisualAssetRole> {
    match value {
        "original" => Ok(VisualAssetRole::Original),
        "thumbnail" => Ok(VisualAssetRole::Thumbnail),
        "preview" => Ok(VisualAssetRole::Preview),
        "sanitized" => Ok(VisualAssetRole::Sanitized),
        "export" => Ok(VisualAssetRole::Export),
        _ => Err(decode_error("visual asset role", value)),
    }
}
