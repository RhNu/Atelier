use super::{
    AppError, AppResult, ArtifactKind, EmbeddedMetadataStatus, EmbeddedMetadataWarning,
    GalleryImageReference, GalleryImageReferenceDto, GalleryImageReferenceTargetDto, GalleryItem,
    GalleryItemDto, GalleryMetadataStatusDto, GalleryMetadataWarningCodeDto,
    GalleryMetadataWarningDto, GalleryPageDto, GalleryQuery, GalleryQueryDto, GallerySafetyDto,
    GallerySafetyLabelDto, GallerySafetyModelEvidenceDto, GallerySafetyOverride,
    GallerySafetyOverrideDto, GallerySafetyRatingScoresDto, GallerySafetyReviewDto,
    GallerySafetyReviewStateDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
    GallerySafetyState, GallerySourceKind, GallerySourceKindDto, ImageReferenceTarget,
    ResourceVariantKind, SafetyAssessment, SafetyLabel, SafetyModelEvidence, SafetyReviewOutcome,
    SafetyRiskBand, VisualAssetDto, VisualAssetRole, resource_ref_to_dto,
};

pub fn gallery_query_to_domain(value: &GalleryQueryDto) -> AppResult<GalleryQuery> {
    Ok(GalleryQuery {
        offset: value.offset,
        limit: value.limit,
        artifact_kind: value
            .artifact_kind
            .as_deref()
            .map(artifact_kind_from_str)
            .transpose()?,
        source_kind: value.source_kind.map(source_kind_to_domain),
        manual_safety_override: value.manual_safety_override.map(safety_override_to_domain),
        safety_label: value.safety_label.map(safety_label_to_domain),
    })
}

pub fn gallery_page_to_dto(
    items: Vec<GalleryItem>,
    offset: usize,
    limit: usize,
    total: usize,
) -> GalleryPageDto {
    GalleryPageDto {
        items: items.into_iter().map(gallery_item_to_dto).collect(),
        total,
        offset,
        limit,
    }
}

pub fn gallery_item_to_dto(value: GalleryItem) -> GalleryItemDto {
    let safety = safety_state_to_dto(&value.safety, value.manual_safety_override);
    GalleryItemDto {
        item_id: value.id.as_str().to_owned(),
        artifact_id: value.artifact_id.as_str().to_owned(),
        artifact_kind: artifact_kind_as_str(value.artifact_kind).to_owned(),
        source_kind: source_kind_to_dto(value.source_kind()),
        primary_resource: resource_ref_to_dto(&value.primary_resource),
        assets: value.assets.iter().map(visual_asset_ref_to_dto).collect(),
        indexed_at_ms: value.indexed_at_ms,
        seed: value.metadata.seed,
        request_seed: value.metadata.request_seed,
        prompt: value.metadata.embedded_prompt.or_else(|| {
            value
                .replay
                .as_ref()
                .and_then(|replay| replay.prompt_snapshot.clone())
        }),
        negative_prompt: value.metadata.embedded_negative_prompt.or_else(|| {
            value
                .replay
                .as_ref()
                .and_then(|replay| replay.negative_prompt_snapshot.clone())
        }),
        embedded_metadata_status: value
            .metadata
            .embedded_metadata_status
            .map(metadata_status_to_dto),
        embedded_metadata_error: value.metadata.embedded_metadata_error,
        embedded_metadata_warnings: value
            .metadata
            .embedded_metadata_warnings
            .into_iter()
            .map(metadata_warning_to_dto)
            .collect(),
        sample_index: value.metadata.sample_index,
        model_name: value.metadata.model_name,
        safety,
        manual_safety_override: value.manual_safety_override.map(safety_override_to_dto),
    }
}

const fn metadata_status_to_dto(value: EmbeddedMetadataStatus) -> GalleryMetadataStatusDto {
    match value {
        EmbeddedMetadataStatus::Parsed => GalleryMetadataStatusDto::Parsed,
        EmbeddedMetadataStatus::NotPresent => GalleryMetadataStatusDto::NotPresent,
        EmbeddedMetadataStatus::UnsupportedFormat => GalleryMetadataStatusDto::UnsupportedFormat,
        EmbeddedMetadataStatus::Invalid => GalleryMetadataStatusDto::Invalid,
    }
}

fn metadata_warning_to_dto(value: EmbeddedMetadataWarning) -> GalleryMetadataWarningDto {
    match value {
        EmbeddedMetadataWarning::InvalidCommentJson => GalleryMetadataWarningDto {
            code: GalleryMetadataWarningCodeDto::InvalidCommentJson,
            keyword: None,
            message: None,
        },
        EmbeddedMetadataWarning::InvalidTextChunk { keyword, message } => {
            GalleryMetadataWarningDto {
                code: GalleryMetadataWarningCodeDto::InvalidTextChunk,
                keyword: Some(keyword),
                message: Some(message),
            }
        }
        EmbeddedMetadataWarning::Unknown(message) => GalleryMetadataWarningDto {
            code: GalleryMetadataWarningCodeDto::Unknown,
            keyword: None,
            message: Some(message),
        },
    }
}

fn safety_state_to_dto(
    value: &GallerySafetyState,
    manual_override: Option<GallerySafetyOverride>,
) -> GallerySafetyDto {
    let manual_label = manual_override.map(|value| match value {
        GallerySafetyOverride::Safe => SafetyLabel::Safe,
        GallerySafetyOverride::Sensitive => SafetyLabel::Sensitive,
        GallerySafetyOverride::Hidden => SafetyLabel::Hidden,
    });
    match value {
        GallerySafetyState::Unscanned => empty_safety_dto(
            GallerySafetyScanStateDto::Unscanned,
            manual_label,
            None,
            None,
        ),
        GallerySafetyState::Failed {
            message,
            attempted_at_ms,
        } => empty_safety_dto(
            GallerySafetyScanStateDto::Failed,
            manual_label,
            Some(message.clone()),
            Some(*attempted_at_ms),
        ),
        GallerySafetyState::Unavailable { message } => empty_safety_dto(
            GallerySafetyScanStateDto::Unavailable,
            manual_label,
            Some(message.clone()),
            None,
        ),
        GallerySafetyState::Scanned(assessment) => {
            safety_assessment_to_dto(assessment, manual_override)
        }
    }
}

fn empty_safety_dto(
    scan_state: GallerySafetyScanStateDto,
    effective_label: Option<SafetyLabel>,
    message: Option<String>,
    attempted_at_ms: Option<u64>,
) -> GallerySafetyDto {
    GallerySafetyDto {
        scan_state,
        risk_band: None,
        auto_label: None,
        effective_label: effective_label.map(safety_label_to_dto),
        policy_id: None,
        policy_version: None,
        primary: None,
        review: None,
        assessed_at_ms: attempted_at_ms,
        message,
    }
}

fn safety_assessment_to_dto(
    value: &SafetyAssessment,
    manual_override: Option<GallerySafetyOverride>,
) -> GallerySafetyDto {
    GallerySafetyDto {
        scan_state: GallerySafetyScanStateDto::Scanned,
        risk_band: Some(safety_risk_band_to_dto(value.risk_band)),
        auto_label: Some(safety_label_to_dto(value.auto_label)),
        effective_label: Some(safety_label_to_dto(value.effective_label(
            manual_override.map(|value| match value {
                GallerySafetyOverride::Safe => SafetyLabel::Safe,
                GallerySafetyOverride::Sensitive => SafetyLabel::Sensitive,
                GallerySafetyOverride::Hidden => SafetyLabel::Hidden,
            }),
        ))),
        policy_id: Some(value.policy_id.clone()),
        policy_version: Some(value.policy_version.clone()),
        primary: Some(safety_model_evidence_to_dto(&value.primary)),
        review: Some(safety_review_to_dto(&value.review)),
        assessed_at_ms: value.assessed_at_ms,
        message: None,
    }
}

fn safety_model_evidence_to_dto(value: &SafetyModelEvidence) -> GallerySafetyModelEvidenceDto {
    GallerySafetyModelEvidenceDto {
        model_id: value.model.id.as_str().to_owned(),
        model_revision: value.model.revision.clone(),
        ratings: GallerySafetyRatingScoresDto {
            general: value.ratings.general.value(),
            sensitive: value.ratings.sensitive.value(),
            questionable: value.ratings.questionable.value(),
            explicit: value.ratings.explicit.value(),
        },
        fused_score: value.fused_score.value(),
    }
}

fn safety_review_to_dto(value: &SafetyReviewOutcome) -> GallerySafetyReviewDto {
    match value {
        SafetyReviewOutcome::NotNeeded => GallerySafetyReviewDto {
            state: GallerySafetyReviewStateDto::NotNeeded,
            model_id: None,
            model_revision: None,
            evidence: None,
            message: None,
        },
        SafetyReviewOutcome::Disabled => GallerySafetyReviewDto {
            state: GallerySafetyReviewStateDto::Disabled,
            model_id: None,
            model_revision: None,
            evidence: None,
            message: None,
        },
        SafetyReviewOutcome::Completed(evidence) => GallerySafetyReviewDto {
            state: GallerySafetyReviewStateDto::Completed,
            model_id: Some(evidence.model.id.as_str().to_owned()),
            model_revision: Some(evidence.model.revision.clone()),
            evidence: Some(safety_model_evidence_to_dto(evidence)),
            message: None,
        },
        SafetyReviewOutcome::Failed { model, message } => GallerySafetyReviewDto {
            state: GallerySafetyReviewStateDto::Failed,
            model_id: Some(model.id.as_str().to_owned()),
            model_revision: Some(model.revision.clone()),
            evidence: None,
            message: Some(message.clone()),
        },
    }
}

const fn safety_risk_band_to_dto(value: SafetyRiskBand) -> GallerySafetyRiskBandDto {
    match value {
        SafetyRiskBand::Low => GallerySafetyRiskBandDto::Low,
        SafetyRiskBand::Medium => GallerySafetyRiskBandDto::Medium,
        SafetyRiskBand::High => GallerySafetyRiskBandDto::High,
    }
}

const fn safety_label_to_dto(value: SafetyLabel) -> GallerySafetyLabelDto {
    match value {
        SafetyLabel::Safe => GallerySafetyLabelDto::Safe,
        SafetyLabel::Sensitive => GallerySafetyLabelDto::Sensitive,
        SafetyLabel::Hidden => GallerySafetyLabelDto::Hidden,
    }
}

const fn safety_label_to_domain(value: GallerySafetyLabelDto) -> SafetyLabel {
    match value {
        GallerySafetyLabelDto::Safe => SafetyLabel::Safe,
        GallerySafetyLabelDto::Sensitive => SafetyLabel::Sensitive,
        GallerySafetyLabelDto::Hidden => SafetyLabel::Hidden,
    }
}

fn visual_asset_ref_to_dto(value: &atelier_artifacts::VisualAssetRef) -> VisualAssetDto {
    VisualAssetDto {
        role: visual_asset_role_as_str(value.role).to_owned(),
        resource: resource_ref_to_dto(&value.resource),
        variant_kind: value
            .variant_kind
            .map(|kind| resource_variant_kind_as_str(kind).to_owned()),
    }
}

const fn artifact_kind_as_str(value: ArtifactKind) -> &'static str {
    match value {
        ArtifactKind::GeneratedImage => "generated_image",
        ArtifactKind::DirectorResult => "director_result",
    }
}

fn artifact_kind_from_str(value: &str) -> AppResult<ArtifactKind> {
    match value {
        "generated_image" => Ok(ArtifactKind::GeneratedImage),
        "director_result" => Ok(ArtifactKind::DirectorResult),
        _ => Err(AppError::new(
            "invalid_request",
            format!("unknown gallery artifact kind `{value}`"),
        )),
    }
}

pub fn gallery_image_reference_to_dto(value: GalleryImageReference) -> GalleryImageReferenceDto {
    let GalleryImageReference {
        item_id,
        artifact_id,
        target,
        asset,
        resource,
    } = value;
    GalleryImageReferenceDto {
        item_id: item_id.as_str().to_owned(),
        artifact_id: artifact_id.as_str().to_owned(),
        target: image_reference_target_to_dto(target),
        resource: resource_ref_to_dto(&resource),
        asset_role: visual_asset_role_as_str(asset.role).to_owned(),
        variant_kind: asset
            .variant_kind
            .map(|kind| resource_variant_kind_as_str(kind).to_owned()),
    }
}

pub const fn image_reference_target_to_domain(
    value: GalleryImageReferenceTargetDto,
) -> ImageReferenceTarget {
    match value {
        GalleryImageReferenceTargetDto::Director => ImageReferenceTarget::Director,
        GalleryImageReferenceTargetDto::ImageToImage => ImageReferenceTarget::ImageToImage,
        GalleryImageReferenceTargetDto::Vibe => ImageReferenceTarget::Vibe,
        GalleryImageReferenceTargetDto::PreciseReference => ImageReferenceTarget::PreciseReference,
    }
}

const fn image_reference_target_to_dto(
    value: ImageReferenceTarget,
) -> GalleryImageReferenceTargetDto {
    match value {
        ImageReferenceTarget::Director => GalleryImageReferenceTargetDto::Director,
        ImageReferenceTarget::ImageToImage => GalleryImageReferenceTargetDto::ImageToImage,
        ImageReferenceTarget::Vibe => GalleryImageReferenceTargetDto::Vibe,
        ImageReferenceTarget::PreciseReference => GalleryImageReferenceTargetDto::PreciseReference,
    }
}

const fn visual_asset_role_as_str(value: VisualAssetRole) -> &'static str {
    match value {
        VisualAssetRole::Original => "original",
        VisualAssetRole::Thumbnail => "thumbnail",
        VisualAssetRole::Preview => "preview",
        VisualAssetRole::Sanitized => "sanitized",
        VisualAssetRole::Export => "export",
    }
}

const fn resource_variant_kind_as_str(value: ResourceVariantKind) -> &'static str {
    match value {
        ResourceVariantKind::Original => "original",
        ResourceVariantKind::Preview => "preview",
        ResourceVariantKind::Thumbnail => "thumbnail",
        ResourceVariantKind::Sanitized => "sanitized",
        ResourceVariantKind::Export => "export",
    }
}

pub const fn source_kind_to_domain(value: GallerySourceKindDto) -> GallerySourceKind {
    match value {
        GallerySourceKindDto::Generation => GallerySourceKind::Generation,
        GallerySourceKindDto::Director => GallerySourceKind::Director,
    }
}

const fn source_kind_to_dto(value: GallerySourceKind) -> GallerySourceKindDto {
    match value {
        GallerySourceKind::Generation => GallerySourceKindDto::Generation,
        GallerySourceKind::Director => GallerySourceKindDto::Director,
    }
}

pub const fn safety_override_to_domain(value: GallerySafetyOverrideDto) -> GallerySafetyOverride {
    match value {
        GallerySafetyOverrideDto::Safe => GallerySafetyOverride::Safe,
        GallerySafetyOverrideDto::Sensitive => GallerySafetyOverride::Sensitive,
        GallerySafetyOverrideDto::Hidden => GallerySafetyOverride::Hidden,
    }
}

const fn safety_override_to_dto(value: GallerySafetyOverride) -> GallerySafetyOverrideDto {
    match value {
        GallerySafetyOverride::Safe => GallerySafetyOverrideDto::Safe,
        GallerySafetyOverride::Sensitive => GallerySafetyOverrideDto::Sensitive,
        GallerySafetyOverride::Hidden => GallerySafetyOverrideDto::Hidden,
    }
}
