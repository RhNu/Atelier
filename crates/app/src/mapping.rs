use atelier_app_api::account::{ApiKeyRecordDto, SubscriptionSummaryDto};
use atelier_app_api::gallery::{
    GalleryImageReferenceDto, GalleryImageReferenceTargetDto, GalleryItemDto,
    GalleryMetadataStatusDto, GalleryMetadataWarningCodeDto, GalleryMetadataWarningDto,
    GalleryPageDto, GalleryQueryDto, GallerySafetyDto, GallerySafetyLabelDto,
    GallerySafetyOverrideDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
    GallerySafetyScoreDto, GallerySourceKindDto, VisualAssetDto,
};
use atelier_app_api::generation::{
    GenerationPlanContextDto, GenerationRequestStatusDto, GenerationStatusDto, ImageFormatDto,
    ImageModelDto, ImageSizeDto, NoiseScheduleDto, QueueDelayDto, QueueDirectiveDto, SamplerDto,
    StreamModeDto, UcPresetDto,
};
use atelier_app_api::history::{
    GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, RunHistoryItemDto, RunHistoryKindDto, RunHistoryOutputDto,
    RunHistoryOutputStateDto, RunHistoryPageDto, RunHistoryQueryDto, RunHistoryStatusDto,
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
    GlobalGallerySettingsDto, GlobalSettingsDto, ImageVariantSettingsDto, WorkspaceSettingsDto,
};
use atelier_app_api::vibe::{
    EnsuredVibeEncodingDto, ExportedVibeDocumentDto, ImportedVibeDocumentsDto,
    VibeDocumentEntryDto, VibeEncodingConfigDto, VibeExportFormatDto, VibeModelDto,
};
use atelier_artifacts::{
    ArtifactKind, EmbeddedMetadataStatus, EmbeddedMetadataWarning, VisualAssetRole,
};
use atelier_gallery::{
    GalleryImageReference, GalleryItem, GalleryQuery, GallerySafetyOverride, GallerySourceKind,
    ImageReferenceTarget,
};
use atelier_generation::{
    GenerationPlanContext, ImageFormat, ImageModel, ImageSize, NoiseSchedule, Sampler, StreamMode,
    UcPreset,
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
use atelier_safety::{SafetyAssessment, SafetyLabel, SafetyRiskBand};
use atelier_secrets::{ApiKeyId, ApiKeyRecord, CreateApiKeyRequest, SecretValue};
use atelier_settings::{
    FrontendLanguage, GenerationDefaults, GlobalFrontendSettings, GlobalGallerySettings,
    GlobalSettings, ImageVariantSettings, WorkspaceSettings,
};
use atelier_vibe::{VibeDocumentEntry, VibeExportFormat, VibeModel};

use crate::{AppError, AppResult};

mod account;
mod gallery;
mod generation;
mod generation_draft;
mod history;
mod prompt;
mod resource;
mod settings;
mod vibe;

pub use account::*;
pub use gallery::*;
pub use generation::*;
pub use generation_draft::*;
pub use history::*;
pub use prompt::*;
pub use resource::*;
pub use settings::*;
pub use vibe::*;

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
