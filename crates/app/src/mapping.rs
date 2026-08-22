use atelier_app_api::account::{ApiKeyRecordDto, SubscriptionSummaryDto};
use atelier_app_api::gallery::{
    GalleryImageReferenceDto, GalleryImageReferenceTargetDto, GalleryItemDto,
    GalleryMetadataStatusDto, GalleryMetadataWarningCodeDto, GalleryMetadataWarningDto,
    GalleryPageDto, GalleryQueryDto, GallerySafetyDto, GallerySafetyLabelDto,
    GallerySafetyModelEvidenceDto, GallerySafetyOverrideDto, GallerySafetyRatingScoresDto,
    GallerySafetyReviewDto, GallerySafetyReviewStateDto, GallerySafetyRiskBandDto,
    GallerySafetyScanStateDto, GallerySourceKindDto, VisualAssetDto,
};
use atelier_app_api::generation::{
    GenerationPlanContextDto, GenerationRequestStatusDto, GenerationStatusDto, ImageFormatDto,
    ImageModelDto, ImageSizeDto, NoiseScheduleDto, PromptStructureDto, QualityPresetDto,
    QueueDelayDto, QueueDirectiveDto, SamplerDto, StreamModeDto, UcPresetDto,
};
use atelier_app_api::history::{
    GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, RunHistoryItemDto, RunHistoryKindDto, RunHistoryOutputDto,
    RunHistoryOutputStateDto, RunHistoryPageDto, RunHistoryQueryDto, RunHistoryStatusDto,
};
use atelier_app_api::image_analysis::{
    ImageAnalysisModelIdDto, ImageAnalysisModelStateDto, ImageAnalysisModelStatusDto,
};
use atelier_app_api::prompt::{
    CompiledPromptDto, LexiconBootstrapDto, LexiconCapabilityStatusDto, LexiconCategoryDto,
    LexiconContentRatingDto, LexiconEntityDetailDto, LexiconEntityKindDto, LexiconFacetDto,
    LexiconGroupSummaryDto, LexiconRelatedEntityDto, LexiconSearchItemDto, LexiconSearchModeDto,
    LexiconSearchPageDto, LexiconSearchRequestDto, LexiconStatsDto, LocalizedLexiconTextDto,
    PromptChunkDto, PromptFunctionTraceEntryDto, PromptPresetBehaviorDto, PromptPresetDto,
    PromptPresetKindDto, PromptTraceDto, UpsertPromptChunkRequestDto, UpsertPromptPresetRequestDto,
};
use atelier_app_api::resource::ResourceRefDto;
use atelier_app_api::settings::{
    FrontendLanguageDto, GenerationDefaultsDto, GlobalFrontendSettingsDto,
    GlobalGallerySettingsDto, GlobalSafetySettingsDto, GlobalSettingsDto, ImageVariantSettingsDto,
    WorkspaceSettingsDto,
};
use atelier_app_api::vibe::{
    EnsuredVibeEncodingDto, ExportedVibeDocumentDto, ImportedVibeDocumentsDto,
    VibeDocumentEntryDto, VibeEncodingConfigDto, VibeExportFormatDto,
};
use atelier_artifacts::{
    ArtifactKind, EmbeddedMetadataStatus, EmbeddedMetadataWarning, VisualAssetRole,
};
use atelier_gallery::{
    GalleryImageReference, GalleryItem, GalleryQuery, GallerySafetyOverride, GallerySafetyState,
    GallerySourceKind, ImageReferenceTarget,
};
use atelier_generation::{
    GenerationPlanContext, ImageFormat, ImageModel, ImageSize, NoiseSchedule, PromptStructure,
    QualityPreset, Sampler, StreamMode, UcPreset,
};
use atelier_image_analysis::{
    ImageAnalysisModelId, ImageAnalysisModelState, ImageAnalysisModelStatus,
};
use atelier_jobs::{
    ActiveJobBatchSnapshot, BatchStatus, GenerationBatchHistoryQuery, GenerationBatchHistoryRecord,
    GenerationBatchHistoryStatus, QueueDelay, QueueDirective, RunHistoryKind, RunHistoryQuery,
    RunHistoryRecord, RunHistoryStatus, RunOutputRecord, RunOutputState,
};
use atelier_kernel::{EnsuredVibeEncoding, ExportedVibeDocument, ImportedVibeDocuments};
use atelier_prompt_lexicon::{
    DanbooruCategory, LexiconBootstrap, LexiconContentRating, LexiconEntityDetail,
    LexiconEntityKind, LexiconSearchFilters, LexiconSearchItem, LexiconSearchMode,
    LexiconSearchPage, LexiconSearchQuery,
};
use atelier_prompt_resources::{
    CompiledPrompt, PromptChunk, PromptChunkId, PromptChunkKey, PromptFunctionTraceEntry,
    PromptPreset, PromptPresetBehavior, PromptPresetId, PromptPresetKind, PromptTrace,
    UpsertPromptChunkRequest, UpsertPromptPresetRequest,
};
use atelier_resource_catalog::{ResourceId, ResourceRef, ResourceVariantKind, VariantId};
use atelier_safety::{
    SafetyAssessment, SafetyLabel, SafetyModelEvidence, SafetyReviewOutcome, SafetyRiskBand,
};
use atelier_secrets::{ApiKeyId, ApiKeyRecord, CreateApiKeyRequest, SecretValue};
use atelier_settings::{
    FrontendLanguage, GenerationDefaults, GlobalFrontendSettings, GlobalGallerySettings,
    GlobalSafetySettings, GlobalSettings, ImageVariantSettings, WorkspaceSettings,
};
use atelier_vibe::{VibeDocumentEntry, VibeExportFormat, VibeModel};

use crate::{AppError, AppResult};

mod account;
mod gallery;
mod generation;
mod generation_draft;
mod history;
mod image_analysis;
mod prompt;
mod resource;
mod settings;
mod vibe;

pub use account::*;
pub use gallery::*;
pub use generation::*;
pub use generation_draft::*;
pub use history::*;
pub use image_analysis::*;
pub use prompt::*;
pub use resource::*;
pub use settings::*;
pub use vibe::*;

pub const fn image_model_to_domain(value: ImageModelDto) -> ImageModel {
    match value {
        ImageModelDto::NaiDiffusion5Full => ImageModel::NaiDiffusion5Full,
        ImageModelDto::NaiDiffusion5Curated => ImageModel::NaiDiffusion5Curated,
        ImageModelDto::NaiDiffusion45Full => ImageModel::NaiDiffusion45Full,
        ImageModelDto::NaiDiffusion45Curated => ImageModel::NaiDiffusion45Curated,
        ImageModelDto::NaiDiffusion4Full => ImageModel::NaiDiffusion4Full,
        ImageModelDto::NaiDiffusion4Curated => ImageModel::NaiDiffusion4Curated,
        ImageModelDto::NaiDiffusion3 => ImageModel::NaiDiffusion3,
        ImageModelDto::NaiDiffusion3Furry => ImageModel::NaiDiffusion3Furry,
    }
}

pub const fn image_model_to_dto(value: ImageModel) -> ImageModelDto {
    match value {
        ImageModel::NaiDiffusion5Full => ImageModelDto::NaiDiffusion5Full,
        ImageModel::NaiDiffusion5Curated => ImageModelDto::NaiDiffusion5Curated,
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
        SamplerDto::KDpmpp2mSde => Sampler::KDpmpp2mSde,
        SamplerDto::KDpmpp2sAncestral => Sampler::KDpmpp2sAncestral,
        SamplerDto::KDpmppSde => Sampler::KDpmppSde,
        SamplerDto::Ddim => Sampler::Ddim,
        SamplerDto::DdimV3 => Sampler::DdimV3,
    }
}

const fn sampler_to_dto(value: Sampler) -> SamplerDto {
    match value {
        Sampler::KEuler => SamplerDto::KEuler,
        Sampler::KEulerAncestral => SamplerDto::KEulerAncestral,
        Sampler::KDpm2 => SamplerDto::KDpm2,
        Sampler::KDpm2Ancestral => SamplerDto::KDpm2Ancestral,
        Sampler::KDpmpp2m => SamplerDto::KDpmpp2m,
        Sampler::KDpmpp2mSde => SamplerDto::KDpmpp2mSde,
        Sampler::KDpmpp2sAncestral => SamplerDto::KDpmpp2sAncestral,
        Sampler::KDpmppSde => SamplerDto::KDpmppSde,
        Sampler::Ddim => SamplerDto::Ddim,
        Sampler::DdimV3 => SamplerDto::DdimV3,
    }
}

pub const fn noise_schedule_to_domain(value: NoiseScheduleDto) -> NoiseSchedule {
    match value {
        NoiseScheduleDto::Native => NoiseSchedule::Native,
        NoiseScheduleDto::Karras => NoiseSchedule::Karras,
        NoiseScheduleDto::Exponential => NoiseSchedule::Exponential,
        NoiseScheduleDto::Polyexponential => NoiseSchedule::Polyexponential,
    }
}

const fn noise_schedule_to_dto(value: NoiseSchedule) -> NoiseScheduleDto {
    match value {
        NoiseSchedule::Native => NoiseScheduleDto::Native,
        NoiseSchedule::Karras => NoiseScheduleDto::Karras,
        NoiseSchedule::Exponential => NoiseScheduleDto::Exponential,
        NoiseSchedule::Polyexponential => NoiseScheduleDto::Polyexponential,
    }
}

pub const fn quality_preset_to_domain(value: QualityPresetDto) -> QualityPreset {
    match value {
        QualityPresetDto::Standard => QualityPreset::Standard,
        QualityPresetDto::Light => QualityPreset::Light,
        QualityPresetDto::None => QualityPreset::None,
    }
}

pub const fn quality_preset_to_dto(value: QualityPreset) -> QualityPresetDto {
    match value {
        QualityPreset::Standard => QualityPresetDto::Standard,
        QualityPreset::Light => QualityPresetDto::Light,
        QualityPreset::None => QualityPresetDto::None,
    }
}

pub const fn prompt_structure_to_dto(value: PromptStructure) -> PromptStructureDto {
    match value {
        PromptStructure::Legacy => PromptStructureDto::Legacy,
        PromptStructure::V4 => PromptStructureDto::V4,
    }
}

pub const fn model_descriptor_to_dto(
    model: ImageModel,
) -> atelier_app_api::generation::ImageModelDescriptorDto {
    let capabilities = model.capabilities();
    atelier_app_api::generation::ImageModelDescriptorDto {
        model: image_model_to_dto(model),
        capabilities: atelier_app_api::generation::ModelCapabilitiesDto {
            prompt_structure: prompt_structure_to_dto(capabilities.prompt_structure),
            params_version: capabilities.params_version,
            default_steps: capabilities.default_steps,
            default_scale: capabilities.default_scale,
            max_characters: capabilities.max_characters,
            supports_vibe_transfer: capabilities.supports_vibe_transfer,
            supports_encoded_vibe: capabilities.supports_encoded_vibe,
            supports_character_reference: capabilities.supports_character_reference,
            supports_character_reference_inpainting: capabilities
                .supports_character_reference_inpainting,
            supports_variety_boost: capabilities.supports_variety_boost,
            supports_inpainting: capabilities.supports_inpainting,
            supports_streaming: capabilities.supports_streaming,
            supports_smea: capabilities.supports_smea,
            supports_dynamic_thresholding: capabilities.supports_dynamic_thresholding,
            uses_v5_extensions: capabilities.uses_v5_extensions,
            supports_light_quality_preset: capabilities.supports_light_quality_preset,
            supports_transparent_background: capabilities.supports_transparent_background,
            variety_sigma_coefficient: capabilities.variety_sigma_coefficient,
            prompt_token_limit: capabilities.prompt_token_limit,
        },
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
        tier: value.tier,
        subscription_active: value.subscription_active,
        v5_usage_is_negative: value.v5_usage_is_negative,
    }
}
