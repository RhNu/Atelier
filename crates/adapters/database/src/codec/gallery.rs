use super::{
    ArtifactId, ArtifactMetadataDto, ArtifactReplayManifestDto, ArtifactSourceDto, DatabaseError,
    DatabaseResult, Deserialize, GalleryItem, GalleryItemId, GallerySafetyState,
    ImageAnalysisModelId, ImageAnalysisModelInfo, ImageRatingScores, ImageSafetyScore,
    JSON_SCHEMA_VERSION, JsonCodec, ResourceRefDto, SafetyAssessment, SafetyLabel,
    SafetyModelEvidence, SafetyReviewOutcome, SafetyRiskBand, Serialize, VisualAssetRefDto,
    VisualAssetRole, artifact_kind_as_str, artifact_kind_from_str, decode_error, ensure_schema,
    safety_override_as_str, safety_override_from_str,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SafetyAssessmentDto {
    resource: ResourceRefDto,
    auto_label: String,
    risk_band: String,
    policy_id: String,
    policy_version: String,
    primary: SafetyModelEvidenceDto,
    review: SafetyReviewOutcomeDto,
    assessed_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SafetyModelEvidenceDto {
    model_id: String,
    model_revision: String,
    ratings: SafetyRatingScoresDto,
    fused_score: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SafetyRatingScoresDto {
    general: f32,
    sensitive: f32,
    questionable: f32,
    explicit_rating: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum SafetyReviewOutcomeDto {
    NotNeeded,
    Disabled,
    Completed {
        evidence: SafetyModelEvidenceDto,
    },
    Failed {
        model_id: String,
        model_revision: String,
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum GallerySafetyStateDto {
    Unscanned,
    Scanned {
        assessment: Box<SafetyAssessmentDto>,
    },
    Failed {
        message: String,
        attempted_at_ms: u64,
    },
    Unavailable {
        message: String,
    },
}

impl From<&GallerySafetyState> for GallerySafetyStateDto {
    fn from(value: &GallerySafetyState) -> Self {
        match value {
            GallerySafetyState::Unscanned => Self::Unscanned,
            GallerySafetyState::Scanned(assessment) => Self::Scanned {
                assessment: Box::new(SafetyAssessmentDto::from(assessment.as_ref())),
            },
            GallerySafetyState::Failed {
                message,
                attempted_at_ms,
            } => Self::Failed {
                message: message.clone(),
                attempted_at_ms: *attempted_at_ms,
            },
            GallerySafetyState::Unavailable { message } => Self::Unavailable {
                message: message.clone(),
            },
        }
    }
}

impl GallerySafetyStateDto {
    fn into_domain(self) -> DatabaseResult<GallerySafetyState> {
        match self {
            Self::Unscanned => Ok(GallerySafetyState::Unscanned),
            Self::Scanned { assessment } => (*assessment)
                .into_domain()
                .map(Box::new)
                .map(GallerySafetyState::Scanned),
            Self::Failed {
                message,
                attempted_at_ms,
            } => Ok(GallerySafetyState::Failed {
                message,
                attempted_at_ms,
            }),
            Self::Unavailable { message } => Ok(GallerySafetyState::Unavailable { message }),
        }
    }
}

impl From<&SafetyAssessment> for SafetyAssessmentDto {
    fn from(value: &SafetyAssessment) -> Self {
        Self {
            resource: ResourceRefDto::from(&value.resource),
            auto_label: safety_label_as_str(value.auto_label).to_owned(),
            risk_band: safety_risk_band_as_str(value.risk_band).to_owned(),
            policy_id: value.policy_id.clone(),
            policy_version: value.policy_version.clone(),
            primary: SafetyModelEvidenceDto::from(&value.primary),
            review: SafetyReviewOutcomeDto::from(&value.review),
            assessed_at_ms: value.assessed_at_ms,
        }
    }
}

impl From<&SafetyModelEvidence> for SafetyModelEvidenceDto {
    fn from(value: &SafetyModelEvidence) -> Self {
        Self {
            model_id: value.model.id.as_str().to_owned(),
            model_revision: value.model.revision.clone(),
            ratings: SafetyRatingScoresDto {
                general: value.ratings.general.value(),
                sensitive: value.ratings.sensitive.value(),
                questionable: value.ratings.questionable.value(),
                explicit_rating: value.ratings.explicit.value(),
            },
            fused_score: value.fused_score.value(),
        }
    }
}

impl From<&SafetyReviewOutcome> for SafetyReviewOutcomeDto {
    fn from(value: &SafetyReviewOutcome) -> Self {
        match value {
            SafetyReviewOutcome::NotNeeded => Self::NotNeeded,
            SafetyReviewOutcome::Disabled => Self::Disabled,
            SafetyReviewOutcome::Completed(evidence) => Self::Completed {
                evidence: SafetyModelEvidenceDto::from(evidence),
            },
            SafetyReviewOutcome::Failed { model, message } => Self::Failed {
                model_id: model.id.as_str().to_owned(),
                model_revision: model.revision.clone(),
                message: message.clone(),
            },
        }
    }
}

impl SafetyModelEvidenceDto {
    fn into_domain(self) -> DatabaseResult<SafetyModelEvidence> {
        Ok(SafetyModelEvidence {
            model: ImageAnalysisModelInfo {
                id: image_analysis_model_id_from_str(&self.model_id)?,
                revision: self.model_revision,
            },
            ratings: ImageRatingScores::new(
                self.ratings.general,
                self.ratings.sensitive,
                self.ratings.questionable,
                self.ratings.explicit_rating,
            )
            .map_err(|error| DatabaseError::new(error.to_string()))?,
            fused_score: ImageSafetyScore::new(self.fused_score)
                .map_err(|error| DatabaseError::new(error.to_string()))?,
        })
    }
}

impl SafetyReviewOutcomeDto {
    fn into_domain(self) -> DatabaseResult<SafetyReviewOutcome> {
        match self {
            Self::NotNeeded => Ok(SafetyReviewOutcome::NotNeeded),
            Self::Disabled => Ok(SafetyReviewOutcome::Disabled),
            Self::Completed { evidence } => {
                Ok(SafetyReviewOutcome::Completed(evidence.into_domain()?))
            }
            Self::Failed {
                model_id,
                model_revision,
                message,
            } => Ok(SafetyReviewOutcome::Failed {
                model: ImageAnalysisModelInfo {
                    id: image_analysis_model_id_from_str(&model_id)?,
                    revision: model_revision,
                },
                message,
            }),
        }
    }
}

impl SafetyAssessmentDto {
    fn into_domain(self) -> DatabaseResult<SafetyAssessment> {
        Ok(SafetyAssessment {
            resource: self.resource.into_domain(),
            auto_label: safety_label_from_str(&self.auto_label)?,
            risk_band: safety_risk_band_from_str(&self.risk_band)?,
            policy_id: self.policy_id,
            policy_version: self.policy_version,
            primary: self.primary.into_domain()?,
            review: self.review.into_domain()?,
            assessed_at_ms: self.assessed_at_ms,
        })
    }
}

const fn safety_label_as_str(value: SafetyLabel) -> &'static str {
    match value {
        SafetyLabel::Safe => "safe",
        SafetyLabel::Sensitive => "sensitive",
        SafetyLabel::Hidden => "hidden",
    }
}

fn safety_label_from_str(value: &str) -> DatabaseResult<SafetyLabel> {
    match value {
        "safe" => Ok(SafetyLabel::Safe),
        "sensitive" => Ok(SafetyLabel::Sensitive),
        "hidden" => Ok(SafetyLabel::Hidden),
        _ => Err(decode_error("safety label", value)),
    }
}

const fn safety_risk_band_as_str(value: SafetyRiskBand) -> &'static str {
    match value {
        SafetyRiskBand::Low => "low",
        SafetyRiskBand::Medium => "medium",
        SafetyRiskBand::High => "high",
    }
}

fn safety_risk_band_from_str(value: &str) -> DatabaseResult<SafetyRiskBand> {
    match value {
        "low" => Ok(SafetyRiskBand::Low),
        "medium" => Ok(SafetyRiskBand::Medium),
        "high" => Ok(SafetyRiskBand::High),
        _ => Err(decode_error("safety risk band", value)),
    }
}

fn image_analysis_model_id_from_str(value: &str) -> DatabaseResult<ImageAnalysisModelId> {
    match value {
        "anime_dbrating" => Ok(ImageAnalysisModelId::AnimeDbRating),
        "wd_swinv2_tagger_v3" => Ok(ImageAnalysisModelId::WdSwinv2TaggerV3),
        _ => Err(decode_error("image analysis model id", value)),
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
    #[serde(default)]
    replay: Option<ArtifactReplayManifestDto>,
    safety: GallerySafetyStateDto,
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
            replay: value.replay.as_ref().map(ArtifactReplayManifestDto::from),
            safety: GallerySafetyStateDto::from(&value.safety),
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
            replay: self.replay.map(Into::into),
            safety: self.safety.into_domain()?,
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
