use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, SubscriptionSummaryDto, UpdateApiKeyRequestDto,
};
use atelier_app_api::director::{
    DirectorToolDto, DirectorToolResultDto, RunDirectorToolRequestDto,
};
use atelier_app_api::event::AppEventDto;
use atelier_app_api::gallery::{GalleryPageDto, GalleryQueryDto, GallerySafetyOverrideDto};
use atelier_app_api::generation::{
    CharacterDto, CharacterReferenceDto, CharacterReferenceTypeDto, ControlNetConfigDto,
    GenerateImageRequestDto, GenerateImageStreamRequestDto, GenerationAnlasEstimateDto,
    GenerationEstimateRequestDto, GenerationStatusDto, GenerationWorkRequestDto, Img2ImgRequestDto,
    QueueDirectiveDto, SubmitGenerationBatchJobDto, SubmitGenerationBatchRequestDto,
    SubmitGenerationRequestDto, UcPresetDto,
};
use atelier_app_api::prompt::{
    CompileGenerationPromptRequestDto, CompilePromptRequestDto,
    CompiledGenerationCharacterPromptDto, CompiledGenerationPromptDto, CompiledPromptDto,
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, DeletePromptPresetRequestDto,
    DeletePromptPresetResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
    ListPromptPresetsRequestDto, PromptChunkDto, PromptChunkPageDto, PromptPresetDto,
    PromptPresetPageDto, UpsertPromptChunkRequestDto, UpsertPromptPresetRequestDto,
};
use atelier_app_api::resource::ImageInputDto;
use atelier_app_api::settings::{
    ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, GetVibeDocumentRequestDto, ImportEmbeddedPngVibeDocumentRequestDto,
    ImportVibeDocumentRequestDto, ImportedVibeDocumentsDto, ListVibeDocumentsRequestDto,
    RenameVibeDocumentRequestDto, SetVibeDocumentHiddenRequestDto, VibeDocumentEntryDto,
    VibeDocumentPageDto,
};
use atelier_app_api::workspace::WorkspaceStatusDto;
use atelier_artifacts::{ArtifactSource, VisualAssetRole};
use atelier_director::{DirectorTool, RunDirectorToolRequest};
use atelier_gallery::{GalleryItemId, GalleryQuery, GallerySourceKind};
use atelier_generation::{
    AnlasEstimate, Character, CharacterPosition, CharacterReference, CharacterReferenceType,
    ControlNetConfig, ControlNetInput, GenerateImageRequest, GenerateImageStreamRequest, ImageSize,
    Img2ImgRequest, plan_generation_request,
};
use atelier_jobs::{
    BatchId, JobId, JobStatus, RunHistoryKind, RunHistoryRecord, RunHistoryRepository,
    RunHistoryStatus, RunOutputRecord, RunOutputState,
};
use atelier_kernel::{
    EnsureVibeEncoding, ExportVibeDocument, GenerationWorkRequest, ImportEmbeddedPngVibeDocument,
    ImportVibeDocument, RunDirectorTool, SubmitGenerationBatch, SubmitGenerationBatchJob,
};
use atelier_prompt_resources::{
    CompileCharacterPromptRequest, CompileGenerationPromptRequest, CompilePromptRequest,
    PromptChunkId, PromptChunkKey, PromptPresetId,
};
use atelier_resource_catalog::ResourceVariantKind;
use atelier_secrets::{ApiKeyId, SecretStore, SecretValue, SecretsErrorKind};
use atelier_vibe::{VibeEncodeSettings, VibeId, VibeSourceIdentity};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::time::{SystemTime, UNIX_EPOCH};

mod history;
mod resource;

pub use history::{
    GenerationHistoryPosition, GenerationHistoryUpdate, HistoryUseCases,
    generation_history_records_from_queue_snapshot, upsert_generation_history_record,
};
pub use resource::ResourceUseCases;

use crate::app::WorkspaceSession;
use crate::mapping::{
    api_key_record_to_dto, compiled_prompt_to_dto, create_api_key_to_domain, ensured_vibe_to_dto,
    exported_vibe_to_dto, gallery_image_reference_to_dto, gallery_item_to_dto, gallery_page_to_dto,
    gallery_query_to_domain, generation_status_to_dto, image_format_to_domain,
    image_model_to_domain, image_reference_target_to_domain, imported_vibes_to_dto,
    noise_schedule_to_domain, plan_context_to_domain, prompt_chunk_to_dto,
    prompt_preset_kind_to_domain, prompt_preset_to_dto, prompt_trace_to_dto,
    queue_directive_to_dto, resource_ref_from_dto, resource_ref_to_dto, safety_override_to_domain,
    sampler_to_domain, stream_mode_to_domain, subscription_to_dto, uc_preset_to_domain,
    upsert_prompt_chunk_to_domain, upsert_prompt_preset_to_domain, vibe_entry_to_dto,
    vibe_format_to_domain, vibe_model_to_domain, workspace_settings_to_domain,
    workspace_settings_to_dto,
};
use crate::{AppError, AppResult};

mod account;
mod director;
mod events;
mod gallery;
mod generation;
mod generation_draft;
mod generation_persistence;
mod generation_support;
mod prompt;
mod settings;
mod vibe;
mod workspace;

pub use account::AccountUseCases;
pub use director::DirectorUseCases;
pub use events::EventsUseCases;
pub use gallery::GalleryUseCases;
pub use generation::GenerationUseCases;
pub use prompt::PromptUseCases;
pub use settings::SettingsUseCases;
pub use vibe::VibeUseCases;
pub use workspace::WorkspaceUseCases;

fn characters_to_domain(value: Vec<CharacterDto>) -> Vec<Character> {
    value
        .into_iter()
        .map(|character| Character {
            prompt: character.prompt,
            negative_prompt: character.negative_prompt,
            position: CharacterPosition {
                x: character.position.x,
                y: character.position.y,
            },
            enabled: character.enabled,
        })
        .collect()
}

const fn character_reference_type_to_domain(
    value: CharacterReferenceTypeDto,
) -> CharacterReferenceType {
    match value {
        CharacterReferenceTypeDto::Character => CharacterReferenceType::Character,
        CharacterReferenceTypeDto::Style => CharacterReferenceType::Style,
        CharacterReferenceTypeDto::CharacterAndStyle => CharacterReferenceType::CharacterAndStyle,
    }
}

const fn director_tool_to_domain(value: DirectorToolDto) -> DirectorTool {
    match value {
        DirectorToolDto::Lineart => DirectorTool::Lineart,
        DirectorToolDto::Sketch => DirectorTool::Sketch,
        DirectorToolDto::BgRemoval => DirectorTool::BgRemoval,
        DirectorToolDto::Emotion => DirectorTool::Emotion,
        DirectorToolDto::Declutter => DirectorTool::Declutter,
        DirectorToolDto::Colorize => DirectorTool::Colorize,
    }
}

fn generation_work_title(work: &GenerationWorkRequestDto) -> Option<String> {
    let prompt = match work {
        GenerationWorkRequestDto::Image(request) => &request.prompt,
        GenerationWorkRequestDto::Stream(request) => &request.base.prompt,
    };
    (!prompt.trim().is_empty()).then(|| prompt.clone())
}

const fn run_history_status_from_job_status(status: JobStatus) -> RunHistoryStatus {
    match status {
        JobStatus::Queued => RunHistoryStatus::Queued,
        JobStatus::Preparing => RunHistoryStatus::Preparing,
        JobStatus::Running => RunHistoryStatus::Running,
        JobStatus::WaitingRetry => RunHistoryStatus::Waiting,
        JobStatus::Blocked => RunHistoryStatus::Paused,
        JobStatus::Succeeded => RunHistoryStatus::Succeeded,
        JobStatus::Failed => RunHistoryStatus::Failed,
        JobStatus::Skipped => RunHistoryStatus::Skipped,
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

fn unix_timestamp_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
