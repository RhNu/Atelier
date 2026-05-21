use nai_atelier_app_api::account::{ApiKeyRecordDto, SubscriptionSummaryDto};
use nai_atelier_app_api::gallery::{
    GalleryImageReferenceDto, GalleryImageReferenceTargetDto, GalleryItemDto, GalleryPageDto,
    GalleryQueryDto, GallerySafetyDto, GallerySafetyLabelDto, GallerySafetyOverrideDto,
    GallerySafetyRiskBandDto, GallerySafetyScanStateDto, GallerySafetyScoreDto,
    GallerySourceKindDto, VisualAssetDto,
};
use nai_atelier_app_api::generation::{
    GenerationPlanContextDto, GenerationStatusDto, ImageFormatDto, ImageModelDto, ImageSizeDto,
    NoiseScheduleDto, QueueDelayDto, QueueDirectiveDto, SamplerDto, StreamModeDto, UcPresetDto,
};
use nai_atelier_app_api::history::{
    RunHistoryItemDto, RunHistoryKindDto, RunHistoryOutputDto, RunHistoryPageDto,
    RunHistoryQueryDto, RunHistoryStatusDto,
};
use nai_atelier_app_api::prompt::{
    CompiledPromptDto, PromptChunkDto, PromptFunctionTraceEntryDto, PromptLexiconCatalogDto,
    PromptLexiconCategorySummaryDto, PromptLexiconEntryDto, PromptLexiconListQueryDto,
    PromptLexiconPageDto, PromptLexiconStatsDto, PromptLexiconSubcategorySummaryDto,
    PromptTraceDto, UpsertPromptChunkRequestDto,
};
use nai_atelier_app_api::resource::ResourceRefDto;
use nai_atelier_app_api::settings::{
    GenerationDefaultsDto, ImageVariantSettingsDto, WorkspaceSettingsDto,
};
use nai_atelier_app_api::vibe::{
    EnsuredVibeEncodingDto, ExportedVibeDocumentDto, ImportedVibeDocumentsDto,
    VibeDocumentEntryDto, VibeExportFormatDto, VibeModelDto,
};
use nai_atelier_artifacts::{ArtifactKind, VisualAssetRole};
use nai_atelier_gallery::{
    GalleryImageReference, GalleryItem, GalleryQuery, GallerySafetyOverride, GallerySourceKind,
    ImageReferenceTarget,
};
use nai_atelier_generation::{
    GenerationPlanContext, ImageFormat, ImageModel, ImageSize, NoiseSchedule, Sampler, StreamMode,
    UcPreset,
};
use nai_atelier_jobs::{
    BatchStatus, QueueDelay, QueueDirective, RunHistoryKind, RunHistoryQuery, RunHistoryRecord,
    RunHistoryStatus, RunOutputRecord,
};
use nai_atelier_kernel::{EnsuredVibeEncoding, ExportedVibeDocument, ImportedVibeDocuments};
use nai_atelier_prompt_lexicon::{
    PromptLexiconCatalog, PromptLexiconEntry, PromptLexiconListPage, PromptLexiconListQuery,
    PromptLexiconMatchField, PromptLexiconMatchRank,
};
use nai_atelier_prompt_resources::{
    CompiledPrompt, PromptChunk, PromptChunkId, PromptChunkKey, PromptFunctionTraceEntry,
    PromptTrace, UpsertPromptChunkRequest,
};
use nai_atelier_resource_catalog::{ResourceId, ResourceRef, ResourceVariantKind, VariantId};
use nai_atelier_safety::{SafetyAssessment, SafetyLabel, SafetyRiskBand};
use nai_atelier_secrets::{ApiKeyId, ApiKeyRecord, CreateApiKeyRequest, SecretValue};
use nai_atelier_settings::{GenerationDefaults, ImageVariantSettings, WorkspaceSettings};
use nai_atelier_vibe::{VibeDocumentEntry, VibeExportFormat, VibeModel};

use crate::{AppError, AppResult};

pub fn api_key_record_to_dto(record: &ApiKeyRecord) -> ApiKeyRecordDto {
    ApiKeyRecordDto {
        id: record.id.as_str().to_owned(),
        display_name: record.display_name.clone(),
        is_active: record.is_active,
    }
}

pub fn create_api_key_to_domain(
    request: nai_atelier_app_api::account::CreateApiKeyRequestDto,
) -> CreateApiKeyRequest {
    CreateApiKeyRequest {
        id: ApiKeyId::new(request.id),
        display_name: request.display_name,
        secret: SecretValue::new(request.secret),
    }
}

pub fn subscription_to_dto(
    value: &nai_atelier_secrets::SubscriptionSummary,
) -> SubscriptionSummaryDto {
    SubscriptionSummaryDto {
        anlas_balance: value.anlas_balance,
        is_opus: value.is_opus,
        tier: value.tier,
        tier_name: value.tier_name.clone(),
        expires_at_ms: value.expires_at_ms,
    }
}

pub fn queue_directive_to_dto(value: QueueDirective) -> QueueDirectiveDto {
    match value {
        QueueDirective::StartJob(id) => QueueDirectiveDto::StartJob {
            job_id: id.as_str().to_owned(),
        },
        QueueDirective::Wait(delay) => QueueDirectiveDto::Wait {
            delay: queue_delay_to_dto(delay),
        },
        QueueDirective::Paused => QueueDirectiveDto::Paused,
        QueueDirective::Idle => QueueDirectiveDto::Idle,
    }
}

pub fn generation_status_to_dto(
    batch: Option<BatchStatus>,
    job: Option<nai_atelier_jobs::JobStatus>,
) -> GenerationStatusDto {
    GenerationStatusDto {
        batch_status: batch.map(|value| batch_status_as_str(value).to_owned()),
        job_status: job.map(|value| job_status_as_str(value).to_owned()),
    }
}

const fn batch_status_as_str(value: BatchStatus) -> &'static str {
    match value {
        BatchStatus::Running => "running",
        BatchStatus::Waiting => "waiting",
        BatchStatus::Paused => "paused",
        BatchStatus::Stopping => "stopping",
        BatchStatus::Succeeded => "succeeded",
        BatchStatus::Stopped => "stopped",
    }
}

const fn job_status_as_str(value: nai_atelier_jobs::JobStatus) -> &'static str {
    match value {
        nai_atelier_jobs::JobStatus::Queued => "queued",
        nai_atelier_jobs::JobStatus::Preparing => "preparing",
        nai_atelier_jobs::JobStatus::Running => "running",
        nai_atelier_jobs::JobStatus::WaitingRetry => "waiting_retry",
        nai_atelier_jobs::JobStatus::Blocked => "blocked",
        nai_atelier_jobs::JobStatus::Succeeded => "succeeded",
        nai_atelier_jobs::JobStatus::Failed => "failed",
        nai_atelier_jobs::JobStatus::Skipped => "skipped",
    }
}

fn queue_delay_to_dto(value: QueueDelay) -> QueueDelayDto {
    QueueDelayDto {
        min_ms: value.min.as_millis().try_into().unwrap_or(u64::MAX),
        max_ms: value.max.as_millis().try_into().unwrap_or(u64::MAX),
    }
}

pub fn workspace_settings_to_dto(value: &WorkspaceSettings) -> WorkspaceSettingsDto {
    WorkspaceSettingsDto {
        generation: generation_defaults_to_dto(&value.generation),
        image_variants: image_variant_settings_to_dto(value.image_variants),
    }
}

pub fn workspace_settings_to_domain(value: &WorkspaceSettingsDto) -> AppResult<WorkspaceSettings> {
    let settings = WorkspaceSettings {
        generation: generation_defaults_to_domain(&value.generation),
        image_variants: image_variant_settings_to_domain(value.image_variants),
    };
    settings.validate()?;
    Ok(settings)
}

fn generation_defaults_to_dto(value: &GenerationDefaults) -> GenerationDefaultsDto {
    GenerationDefaultsDto {
        model: image_model_to_dto(value.model),
        size: ImageSizeDto {
            width: value.size.width,
            height: value.size.height,
        },
        quality: value.quality,
        uc_preset: uc_preset_to_dto(value.uc_preset),
        steps: value.steps,
        scale: value.scale,
        sampler: sampler_to_dto(value.sampler),
        noise_schedule: noise_schedule_to_dto(value.noise_schedule),
        seed: value.seed,
        n_samples: value.n_samples,
        cfg_rescale: value.cfg_rescale,
        variety_boost: value.variety_boost,
        image_format: value.image_format.map(image_format_to_dto),
        strict_mode: value.strict_mode,
    }
}

fn generation_defaults_to_domain(value: &GenerationDefaultsDto) -> GenerationDefaults {
    GenerationDefaults {
        model: image_model_to_domain(value.model),
        size: ImageSize {
            width: value.size.width,
            height: value.size.height,
        },
        quality: value.quality,
        uc_preset: uc_preset_to_domain(value.uc_preset),
        steps: value.steps,
        scale: value.scale,
        sampler: sampler_to_domain(value.sampler),
        noise_schedule: noise_schedule_to_domain(value.noise_schedule),
        seed: value.seed,
        n_samples: value.n_samples,
        cfg_rescale: value.cfg_rescale,
        variety_boost: value.variety_boost,
        image_format: value.image_format.map(image_format_to_domain),
        strict_mode: value.strict_mode,
    }
}

const fn image_variant_settings_to_dto(value: ImageVariantSettings) -> ImageVariantSettingsDto {
    ImageVariantSettingsDto {
        thumbnail_long_edge: value.thumbnail_long_edge,
        preview_long_edge: value.preview_long_edge,
    }
}

const fn image_variant_settings_to_domain(value: ImageVariantSettingsDto) -> ImageVariantSettings {
    ImageVariantSettings {
        thumbnail_long_edge: value.thumbnail_long_edge,
        preview_long_edge: value.preview_long_edge,
    }
}

pub const fn image_model_to_domain(value: ImageModelDto) -> ImageModel {
    match value {
        ImageModelDto::NaiDiffusion45Full => ImageModel::NaiDiffusion45Full,
        ImageModelDto::NaiDiffusion45Curated => ImageModel::NaiDiffusion45Curated,
        ImageModelDto::NaiDiffusion4Full => ImageModel::NaiDiffusion4Full,
        ImageModelDto::NaiDiffusion4Curated => ImageModel::NaiDiffusion4Curated,
        ImageModelDto::NaiDiffusion3 => ImageModel::NaiDiffusion3,
        ImageModelDto::NaiDiffusion3Furry => ImageModel::NaiDiffusion3Furry,
    }
}

const fn image_model_to_dto(value: ImageModel) -> ImageModelDto {
    match value {
        ImageModel::NaiDiffusion45Full => ImageModelDto::NaiDiffusion45Full,
        ImageModel::NaiDiffusion45Curated => ImageModelDto::NaiDiffusion45Curated,
        ImageModel::NaiDiffusion4Full => ImageModelDto::NaiDiffusion4Full,
        ImageModel::NaiDiffusion4Curated => ImageModelDto::NaiDiffusion4Curated,
        ImageModel::NaiDiffusion3 => ImageModelDto::NaiDiffusion3,
        ImageModel::NaiDiffusion3Furry => ImageModelDto::NaiDiffusion3Furry,
    }
}

pub const fn sampler_to_domain(value: SamplerDto) -> Sampler {
    match value {
        SamplerDto::KEuler => Sampler::KEuler,
        SamplerDto::KEulerAncestral => Sampler::KEulerAncestral,
        SamplerDto::KDpm2 => Sampler::KDpm2,
        SamplerDto::KDpm2Ancestral => Sampler::KDpm2Ancestral,
        SamplerDto::KDpmpp2m => Sampler::KDpmpp2m,
        SamplerDto::KDpmpp2sAncestral => Sampler::KDpmpp2sAncestral,
        SamplerDto::KDpmppSde => Sampler::KDpmppSde,
        SamplerDto::Ddim => Sampler::Ddim,
    }
}

const fn sampler_to_dto(value: Sampler) -> SamplerDto {
    match value {
        Sampler::KEuler => SamplerDto::KEuler,
        Sampler::KEulerAncestral => SamplerDto::KEulerAncestral,
        Sampler::KDpm2 => SamplerDto::KDpm2,
        Sampler::KDpm2Ancestral => SamplerDto::KDpm2Ancestral,
        Sampler::KDpmpp2m => SamplerDto::KDpmpp2m,
        Sampler::KDpmpp2sAncestral => SamplerDto::KDpmpp2sAncestral,
        Sampler::KDpmppSde => SamplerDto::KDpmppSde,
        Sampler::Ddim => SamplerDto::Ddim,
    }
}

pub const fn noise_schedule_to_domain(value: NoiseScheduleDto) -> NoiseSchedule {
    match value {
        NoiseScheduleDto::Karras => NoiseSchedule::Karras,
        NoiseScheduleDto::Exponential => NoiseSchedule::Exponential,
        NoiseScheduleDto::Polyexponential => NoiseSchedule::Polyexponential,
    }
}

const fn noise_schedule_to_dto(value: NoiseSchedule) -> NoiseScheduleDto {
    match value {
        NoiseSchedule::Karras => NoiseScheduleDto::Karras,
        NoiseSchedule::Exponential => NoiseScheduleDto::Exponential,
        NoiseSchedule::Polyexponential => NoiseScheduleDto::Polyexponential,
    }
}

pub const fn uc_preset_to_domain(value: UcPresetDto) -> UcPreset {
    match value {
        UcPresetDto::Heavy => UcPreset::Heavy,
        UcPresetDto::Light => UcPreset::Light,
        UcPresetDto::FurryFocus => UcPreset::FurryFocus,
        UcPresetDto::HumanFocus => UcPreset::HumanFocus,
        UcPresetDto::None => UcPreset::None,
    }
}

const fn uc_preset_to_dto(value: UcPreset) -> UcPresetDto {
    match value {
        UcPreset::Heavy => UcPresetDto::Heavy,
        UcPreset::Light => UcPresetDto::Light,
        UcPreset::FurryFocus => UcPresetDto::FurryFocus,
        UcPreset::HumanFocus => UcPresetDto::HumanFocus,
        UcPreset::None => UcPresetDto::None,
    }
}

pub const fn image_format_to_domain(value: ImageFormatDto) -> ImageFormat {
    match value {
        ImageFormatDto::Png => ImageFormat::Png,
        ImageFormatDto::Webp => ImageFormat::Webp,
    }
}

const fn image_format_to_dto(value: ImageFormat) -> ImageFormatDto {
    match value {
        ImageFormat::Png => ImageFormatDto::Png,
        ImageFormat::Webp => ImageFormatDto::Webp,
    }
}

pub const fn stream_mode_to_domain(value: StreamModeDto) -> StreamMode {
    match value {
        StreamModeDto::Sse => StreamMode::Sse,
    }
}

pub const fn plan_context_to_domain(value: GenerationPlanContextDto) -> GenerationPlanContext {
    GenerationPlanContext {
        request_count: value.request_count,
        pending_vibe_encode_count: value.pending_vibe_encode_count,
        is_opus: value.is_opus,
    }
}

pub fn prompt_chunk_to_dto(chunk: &PromptChunk) -> PromptChunkDto {
    PromptChunkDto {
        chunk_id: chunk.id.as_str().to_owned(),
        key: chunk.key.as_str().to_owned(),
        content: chunk.content.clone(),
        category: chunk.category.clone(),
        description: chunk.description.clone(),
        preview: chunk.preview_thumb.as_ref().map(resource_ref_to_dto),
        created_at_ms: chunk.created_at_ms,
        updated_at_ms: chunk.updated_at_ms,
    }
}

pub fn upsert_prompt_chunk_to_domain(
    request: UpsertPromptChunkRequestDto,
) -> AppResult<UpsertPromptChunkRequest> {
    Ok(UpsertPromptChunkRequest {
        chunk_id: request.chunk_id.map(PromptChunkId::new),
        key: PromptChunkKey::parse(&request.key)?,
        content: request.content,
        category: request.category,
        description: request.description,
        preview_thumb: request.preview.map(resource_ref_from_dto),
    })
}

pub fn compiled_prompt_to_dto(value: &CompiledPrompt) -> CompiledPromptDto {
    CompiledPromptDto {
        expanded_prompt: value.expanded_prompt.clone(),
        trace: prompt_trace_to_dto(&value.trace),
    }
}

fn prompt_trace_to_dto(value: &PromptTrace) -> PromptTraceDto {
    PromptTraceDto {
        raw_prompt: value.raw_prompt.clone(),
        expanded_prompt: value.expanded_prompt.clone(),
        function_calls: value
            .function_calls
            .iter()
            .map(trace_entry_to_dto)
            .collect(),
    }
}

fn trace_entry_to_dto(value: &PromptFunctionTraceEntry) -> PromptFunctionTraceEntryDto {
    PromptFunctionTraceEntryDto {
        function_name: value.function_name.clone(),
        raw_call: value.raw_call.clone(),
        resolved_arguments: value.resolved_arguments.clone(),
        result_text: value.result_text.clone(),
        depth: value.depth,
        call_chain: value.call_chain.clone(),
    }
}

pub fn lexicon_catalog_to_dto(value: PromptLexiconCatalog) -> PromptLexiconCatalogDto {
    PromptLexiconCatalogDto {
        stats: PromptLexiconStatsDto {
            total_tags: value.stats.total_tags,
            categorized_tags: value.stats.categorized_tags,
            uncategorized_tags: value.stats.uncategorized_tags,
            matched_weights: value.stats.matched_weights,
            total_translations: value.stats.total_translations,
            tags_with_aliases: value.stats.tags_with_aliases,
            max_aliases_per_tag: value.stats.max_aliases_per_tag,
            source_count: value.stats.source_count,
            manifest_version: value.stats.manifest_version,
            primary_from_category_json: value.stats.primary_from_category_json,
            primary_from_manifest_sources: value.stats.primary_from_manifest_sources,
            primary_fallback_to_tag: value.stats.primary_fallback_to_tag,
        },
        categories: value
            .categories
            .into_iter()
            .map(|category| PromptLexiconCategorySummaryDto {
                name: category.name,
                tag_count: category.tag_count,
                subcategory_count: category.subcategory_count,
                subcategories: category
                    .subcategories
                    .into_iter()
                    .map(|subcategory| PromptLexiconSubcategorySummaryDto {
                        name: subcategory.name,
                        tag_count: subcategory.tag_count,
                    })
                    .collect(),
            })
            .collect(),
    }
}

pub fn lexicon_query_to_domain(value: PromptLexiconListQueryDto) -> PromptLexiconListQuery {
    PromptLexiconListQuery {
        query: value.query,
        category: value.category,
        subcategory: value.subcategory,
        limit: value.limit,
        offset: value.offset,
    }
}

pub fn lexicon_page_to_dto(value: PromptLexiconListPage) -> PromptLexiconPageDto {
    PromptLexiconPageDto {
        items: value.items.into_iter().map(lexicon_entry_to_dto).collect(),
        total: value.total,
        offset: value.offset,
        limit: value.limit,
    }
}

pub fn lexicon_search_to_page(
    value: nai_atelier_prompt_lexicon::PromptLexiconSearchResult,
    limit: usize,
) -> PromptLexiconPageDto {
    PromptLexiconPageDto {
        items: value.items.into_iter().map(lexicon_entry_to_dto).collect(),
        total: value.total,
        offset: 0,
        limit,
    }
}

fn lexicon_entry_to_dto(value: PromptLexiconEntry) -> PromptLexiconEntryDto {
    PromptLexiconEntryDto {
        tag: value.tag,
        weight: value.weight,
        category: value.category,
        subcategory: value.subcategory,
        primary_translation: value.primary_translation,
        matched_translation: value.matched_translation,
        match_field: match_field_as_str(value.match_field).to_owned(),
        match_rank: match_rank_as_str(value.match_rank).to_owned(),
    }
}

const fn match_field_as_str(value: PromptLexiconMatchField) -> &'static str {
    match value {
        PromptLexiconMatchField::Tag => "tag",
        PromptLexiconMatchField::PrimaryTranslation => "primary_translation",
        PromptLexiconMatchField::Alias => "alias",
    }
}

const fn match_rank_as_str(value: PromptLexiconMatchRank) -> &'static str {
    match value {
        PromptLexiconMatchRank::Exact => "exact",
        PromptLexiconMatchRank::Prefix => "prefix",
        PromptLexiconMatchRank::Substring => "substring",
    }
}

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

pub fn imported_vibes_to_dto(value: ImportedVibeDocuments) -> ImportedVibeDocumentsDto {
    ImportedVibeDocumentsDto {
        entries: value.entries.into_iter().map(vibe_entry_to_dto).collect(),
    }
}

pub fn exported_vibe_to_dto(value: ExportedVibeDocument) -> ExportedVibeDocumentDto {
    ExportedVibeDocumentDto {
        file_extension: value.document.file_extension.to_owned(),
        content: value.document.content,
    }
}

pub fn ensured_vibe_to_dto(value: &EnsuredVibeEncoding) -> EnsuredVibeEncodingDto {
    EnsuredVibeEncodingDto {
        resource: resource_ref_to_dto(&value.record.resource),
        created: value.created,
    }
}

fn vibe_entry_to_dto(value: VibeDocumentEntry) -> VibeDocumentEntryDto {
    VibeDocumentEntryDto {
        vibe_id: value.summary.document_id.as_str().to_owned(),
        display_name: value.summary.display_name,
        has_image: value.summary.has_image,
        available_model_keys: value.summary.available_model_keys,
        document: resource_ref_to_dto(&value.resources.document),
        source_image: value
            .resources
            .source_image
            .as_ref()
            .map(resource_ref_to_dto),
        preview: value.resources.preview.as_ref().map(resource_ref_to_dto),
        encodings: value
            .resources
            .encodings
            .iter()
            .map(resource_ref_to_dto)
            .collect(),
    }
}

pub const fn vibe_model_to_domain(value: VibeModelDto) -> VibeModel {
    match value {
        VibeModelDto::NaiDiffusion45Full => VibeModel::NaiDiffusion45Full,
        VibeModelDto::NaiDiffusion45Curated => VibeModel::NaiDiffusion45Curated,
        VibeModelDto::NaiDiffusion4Full => VibeModel::NaiDiffusion4Full,
        VibeModelDto::NaiDiffusion4Curated => VibeModel::NaiDiffusion4Curated,
        VibeModelDto::NaiDiffusion3 => VibeModel::NaiDiffusion3,
        VibeModelDto::NaiDiffusion3Furry => VibeModel::NaiDiffusion3Furry,
    }
}

pub const fn vibe_format_to_domain(value: VibeExportFormatDto) -> VibeExportFormat {
    match value {
        VibeExportFormatDto::Naiv4vibe => VibeExportFormat::Naiv4vibe,
        VibeExportFormatDto::Naiv4vibebundle => VibeExportFormat::Naiv4vibebundle,
    }
}

pub fn resource_ref_to_dto(value: &ResourceRef) -> ResourceRefDto {
    ResourceRefDto {
        id: value.id.as_str().to_owned(),
        variant_id: value.variant_id.as_ref().map(|id| id.as_str().to_owned()),
    }
}

pub fn resource_ref_from_dto(value: ResourceRefDto) -> ResourceRef {
    ResourceRef::new(
        ResourceId::new(value.id),
        value.variant_id.map(VariantId::new),
    )
}

pub const fn run_history_kind_to_dto(value: RunHistoryKind) -> RunHistoryKindDto {
    match value {
        RunHistoryKind::Generation => RunHistoryKindDto::Generation,
        RunHistoryKind::Director => RunHistoryKindDto::Director,
    }
}

pub const fn run_history_kind_to_domain(value: RunHistoryKindDto) -> RunHistoryKind {
    match value {
        RunHistoryKindDto::Generation => RunHistoryKind::Generation,
        RunHistoryKindDto::Director => RunHistoryKind::Director,
    }
}

pub const fn run_history_status_to_dto(value: RunHistoryStatus) -> RunHistoryStatusDto {
    match value {
        RunHistoryStatus::Queued => RunHistoryStatusDto::Queued,
        RunHistoryStatus::Preparing => RunHistoryStatusDto::Preparing,
        RunHistoryStatus::Running => RunHistoryStatusDto::Running,
        RunHistoryStatus::Waiting => RunHistoryStatusDto::Waiting,
        RunHistoryStatus::Paused => RunHistoryStatusDto::Paused,
        RunHistoryStatus::Succeeded => RunHistoryStatusDto::Succeeded,
        RunHistoryStatus::Failed => RunHistoryStatusDto::Failed,
        RunHistoryStatus::Skipped => RunHistoryStatusDto::Skipped,
        RunHistoryStatus::Stopped => RunHistoryStatusDto::Stopped,
    }
}

pub const fn run_history_status_to_domain(value: RunHistoryStatusDto) -> RunHistoryStatus {
    match value {
        RunHistoryStatusDto::Queued => RunHistoryStatus::Queued,
        RunHistoryStatusDto::Preparing => RunHistoryStatus::Preparing,
        RunHistoryStatusDto::Running => RunHistoryStatus::Running,
        RunHistoryStatusDto::Waiting => RunHistoryStatus::Waiting,
        RunHistoryStatusDto::Paused => RunHistoryStatus::Paused,
        RunHistoryStatusDto::Succeeded => RunHistoryStatus::Succeeded,
        RunHistoryStatusDto::Failed => RunHistoryStatus::Failed,
        RunHistoryStatusDto::Skipped => RunHistoryStatus::Skipped,
        RunHistoryStatusDto::Stopped => RunHistoryStatus::Stopped,
    }
}

pub fn run_history_query_to_domain(value: &RunHistoryQueryDto) -> RunHistoryQuery {
    RunHistoryQuery {
        offset: value.offset,
        limit: value.limit,
        kind: value.kind.map(run_history_kind_to_domain),
        status: value.status.map(run_history_status_to_domain),
    }
}

pub fn run_history_item_to_dto(
    record: RunHistoryRecord,
    outputs: Vec<RunOutputRecord>,
) -> RunHistoryItemDto {
    RunHistoryItemDto {
        run_id: record.run_id,
        kind: run_history_kind_to_dto(record.kind),
        status: run_history_status_to_dto(record.status),
        batch_id: record.batch_id,
        job_id: record.job_id,
        origin_run_id: record.origin_run_id,
        title: record.title,
        last_error: record.last_error,
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
        completed_at_ms: record.completed_at_ms,
        recoverable: record.recoverable,
        outputs: outputs.into_iter().map(run_output_to_dto).collect(),
    }
}

pub const fn run_history_page_to_dto(
    items: Vec<RunHistoryItemDto>,
    offset: usize,
    limit: usize,
    total: usize,
) -> RunHistoryPageDto {
    RunHistoryPageDto {
        items,
        total,
        offset,
        limit,
    }
}

pub fn run_output_to_dto(value: RunOutputRecord) -> RunHistoryOutputDto {
    RunHistoryOutputDto {
        artifact_id: value.artifact_id,
        item_id: value.item_id,
        resource: ResourceRefDto {
            id: value.resource_id,
            variant_id: value.variant_id,
        },
        asset_role: value.asset_role,
        variant_kind: value.variant_kind,
    }
}
