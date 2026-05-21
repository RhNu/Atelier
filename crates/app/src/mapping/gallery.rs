use super::{
    AppError, AppResult, ArtifactKind, GalleryImageReference, GalleryImageReferenceDto,
    GalleryImageReferenceTargetDto, GalleryItem, GalleryItemDto, GalleryPageDto, GalleryQuery,
    GalleryQueryDto, GallerySafetyDto, GallerySafetyLabelDto, GallerySafetyOverride,
    GallerySafetyOverrideDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
    GallerySafetyScoreDto, GallerySourceKind, GallerySourceKindDto, ImageReferenceTarget,
    ResourceVariantKind, SafetyAssessment, SafetyLabel, SafetyRiskBand, VisualAssetDto,
    VisualAssetRole, resource_ref_to_dto,
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
    let safety = value
        .safety_assessment
        .as_ref()
        .map(|assessment| safety_assessment_to_dto(assessment, value.manual_safety_override));
    GalleryItemDto {
        item_id: value.id.as_str().to_owned(),
        artifact_id: value.artifact_id.as_str().to_owned(),
        artifact_kind: artifact_kind_as_str(value.artifact_kind).to_owned(),
        source_kind: source_kind_to_dto(value.source_kind()),
        primary_resource: resource_ref_to_dto(&value.primary_resource),
        assets: value.assets.iter().map(visual_asset_ref_to_dto).collect(),
        indexed_at_ms: value.indexed_at_ms,
        seed: value.metadata.seed,
        sample_index: value.metadata.sample_index,
        model_name: value.metadata.model_name,
        safety,
        manual_safety_override: value.manual_safety_override.map(safety_override_to_dto),
    }
}

fn safety_assessment_to_dto(
    value: &SafetyAssessment,
    manual_override: Option<GallerySafetyOverride>,
) -> GallerySafetyDto {
    GallerySafetyDto {
        scan_state: GallerySafetyScanStateDto::Scanned,
        risk_band: Some(safety_risk_band_to_dto(value.risk_band())),
        auto_label: Some(safety_label_to_dto(value.auto_label())),
        effective_label: safety_label_to_dto(value.effective_label(manual_override.map(|value| {
            match value {
                GallerySafetyOverride::Safe => SafetyLabel::Safe,
                GallerySafetyOverride::Sensitive => SafetyLabel::Sensitive,
                GallerySafetyOverride::Hidden => SafetyLabel::Hidden,
            }
        }))),
        nsfw_score: Some(value.score.value()),
        safe_score: value
            .safe_score
            .map(nai_atelier_safety::ImageSafetyScore::value),
        raw_scores: value
            .raw_scores
            .iter()
            .map(|score| GallerySafetyScoreDto {
                label: score.label.clone(),
                score: score.score.value(),
            })
            .collect(),
        model_id: value.scorer_label.clone(),
        scorer_version: value.scorer_version.clone(),
        assessed_at_ms: value.assessed_at_ms,
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

fn visual_asset_ref_to_dto(value: &nai_atelier_artifacts::VisualAssetRef) -> VisualAssetDto {
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
        ArtifactKind::ImportedImage => "imported_image",
    }
}

fn artifact_kind_from_str(value: &str) -> AppResult<ArtifactKind> {
    match value {
        "generated_image" => Ok(ArtifactKind::GeneratedImage),
        "director_result" => Ok(ArtifactKind::DirectorResult),
        "imported_image" => Ok(ArtifactKind::ImportedImage),
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
        GallerySourceKindDto::Import => GallerySourceKind::Import,
    }
}

const fn source_kind_to_dto(value: GallerySourceKind) -> GallerySourceKindDto {
    match value {
        GallerySourceKind::Generation => GallerySourceKindDto::Generation,
        GallerySourceKind::Director => GallerySourceKindDto::Director,
        GallerySourceKind::Import => GallerySourceKindDto::Import,
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
