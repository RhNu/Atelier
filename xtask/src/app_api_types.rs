use std::fs;
use std::path::{Path, PathBuf};

use atelier_app_api::{
    account::{
        ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
        ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, SubscriptionSummaryDto,
        UpdateApiKeyRequestDto,
    },
    director::{DirectorToolDto, DirectorToolResultDto, RunDirectorToolRequestDto},
    error::ErrorEnvelopeDto,
    event::{AppEventDto, AppEventKindDto, AppEventPageDto, EventsSinceRequestDto},
    gallery::{
        DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryImageReferenceDto,
        GalleryImageReferenceRequestDto, GalleryImageReferenceTargetDto, GalleryItemDto,
        GalleryPageDto, GalleryQueryDto, GallerySafetyDto, GallerySafetyLabelDto,
        GallerySafetyOverrideDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
        GallerySafetyScoreDto, GallerySourceKindDto, SetGallerySafetyOverrideRequestDto,
        VisualAssetDto,
    },
    generation::{
        CharacterDto, CharacterPositionDto, CharacterReferenceDto, CharacterReferenceTypeDto,
        ControlNetConfigDto, ControlNetInputDto, GenerateImageRequestDto,
        GenerateImageStreamRequestDto, GenerationAnlasEstimateDto, GenerationEstimateRequestDto,
        GenerationPlanContextDto, GenerationStatusDto, GenerationStatusQueryDto,
        GenerationWorkRequestDto, ImageFormatDto, ImageModelDto, ImageSizeDto, Img2ImgRequestDto,
        NoiseScheduleDto, QueueDelayDto, QueueDirectiveDto, RunGenerationJobRequestDto, SamplerDto,
        StreamModeDto, SubmitGenerationBatchJobDto, SubmitGenerationBatchRequestDto,
        SubmitGenerationRequestDto, UcPresetDto,
    },
    history::{
        DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
        RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto,
        RunHistoryItemDto, RunHistoryKindDto, RunHistoryOutputDto, RunHistoryPageDto,
        RunHistoryQueryDto, RunHistoryStatusDto,
    },
    pagination::{PageInfoDto, PageQueryDto},
    prompt::{
        CompileGenerationCharacterPromptDto, CompileGenerationPromptRequestDto,
        CompilePromptRequestDto, CompiledGenerationCharacterPromptDto, CompiledGenerationPromptDto,
        CompiledPromptDto, DeletePromptChunkRequestDto, DeletePromptChunkResponseDto,
        DeletePromptPresetRequestDto, DeletePromptPresetResponseDto, GetPromptChunkRequestDto,
        ListPromptChunksRequestDto, ListPromptPresetsRequestDto, PromptChunkDto,
        PromptChunkPageDto, PromptFunctionTraceEntryDto, PromptLexiconCatalogDto,
        PromptLexiconCategorySummaryDto, PromptLexiconEntryDto, PromptLexiconListQueryDto,
        PromptLexiconPageDto, PromptLexiconSearchQueryDto, PromptLexiconStatsDto,
        PromptLexiconSubcategorySummaryDto, PromptPresetDto, PromptPresetKindDto,
        PromptPresetPageDto, PromptTraceDto, UpsertPromptChunkRequestDto,
        UpsertPromptPresetRequestDto,
    },
    resource::{
        GetResourceImageRequestDto, ImageInputDto, ImageResourceKindDto,
        ImportImageResourceRequestDto, ImportImageResourceResponseDto, ResourceImageDto,
        ResourceRefDto, SaveResourceImageRequestDto,
    },
    settings::{
        GenerationDefaultsDto, ImageVariantSettingsDto, ResetWorkspaceSettingsResponseDto,
        UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
    },
    vibe::{
        EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
        ExportedVibeDocumentDto, GetVibeDocumentRequestDto,
        ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
        ImportedVibeDocumentsDto, ListVibeDocumentsRequestDto, RenameVibeDocumentRequestDto,
        SetVibeDocumentHiddenRequestDto, VibeDocumentEntryDto, VibeDocumentPageDto,
        VibeEncodingConfigDto, VibeExportFormatDto, VibeModelDto,
    },
    workspace::{CloseWorkspaceResponseDto, OpenWorkspaceRequestDto, WorkspaceStatusDto},
};
use ts_rs::{Config, TS};

macro_rules! export_types {
    ($config:expr, $($type:ty),+ $(,)?) => {{
        $(export_type::<$type>($config)?;)+
        Ok(())
    }};
}

pub struct AppApiTypeExportConfig {
    pub out_dir: PathBuf,
}

impl AppApiTypeExportConfig {
    #[must_use]
    pub fn default_for_workspace(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            out_dir: workspace_root
                .as_ref()
                .join("apps")
                .join("desktop")
                .join("src")
                .join("types")
                .join("generated"),
        }
    }
}

/// Exports frontend-facing `app-api` DTOs as TypeScript bindings.
///
/// # Errors
/// Returns an error when the output directory cannot be prepared or a type fails to export.
pub fn export_app_api_types(config: &AppApiTypeExportConfig) -> Result<(), String> {
    prepare_output_dir(&config.out_dir)?;
    let ts_config = Config::new()
        .with_large_int("number")
        .with_out_dir(&config.out_dir);

    export_account_types(&ts_config)?;
    export_director_types(&ts_config)?;
    export_event_types(&ts_config)?;
    export_gallery_types(&ts_config)?;
    export_generation_types(&ts_config)?;
    export_history_types(&ts_config)?;
    export_pagination_types(&ts_config)?;
    export_prompt_types(&ts_config)?;
    export_resource_types(&ts_config)?;
    export_settings_types(&ts_config)?;
    export_vibe_types(&ts_config)?;
    export_workspace_types(&ts_config)?;

    write_index_file(&config.out_dir)?;
    Ok(())
}

fn export_account_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        CreateApiKeyRequestDto,
        UpdateApiKeyRequestDto,
        DeleteApiKeyRequestDto,
        DeleteApiKeyResponseDto,
        SetActiveApiKeyRequestDto,
        ProbeApiKeyRequestDto,
        ApiKeyRecordDto,
        SubscriptionSummaryDto,
    )
}

fn export_director_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        DirectorToolDto,
        RunDirectorToolRequestDto,
        DirectorToolResultDto,
    )
}

fn export_event_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        ErrorEnvelopeDto,
        AppEventDto,
        EventsSinceRequestDto,
        AppEventPageDto,
        AppEventKindDto,
    )
}

fn export_gallery_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        GallerySafetyOverrideDto,
        GallerySourceKindDto,
        GalleryQueryDto,
        GalleryPageDto,
        GalleryItemDto,
        GallerySafetyScanStateDto,
        GallerySafetyRiskBandDto,
        GallerySafetyLabelDto,
        GallerySafetyScoreDto,
        GallerySafetyDto,
        VisualAssetDto,
        DeleteGalleryItemsRequestDto,
        DeleteGalleryItemsResponseDto,
        SetGallerySafetyOverrideRequestDto,
        GalleryImageReferenceTargetDto,
        GalleryImageReferenceRequestDto,
        GalleryImageReferenceDto,
    )
}

fn export_generation_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        ImageModelDto,
        ImageSizeDto,
        SamplerDto,
        NoiseScheduleDto,
        UcPresetDto,
        ImageFormatDto,
        StreamModeDto,
        Img2ImgRequestDto,
        ControlNetInputDto,
        ControlNetConfigDto,
        CharacterReferenceTypeDto,
        CharacterReferenceDto,
        CharacterPositionDto,
        CharacterDto,
        GenerateImageRequestDto,
        GenerateImageStreamRequestDto,
        GenerationWorkRequestDto,
        GenerationPlanContextDto,
        SubmitGenerationRequestDto,
        SubmitGenerationBatchJobDto,
        SubmitGenerationBatchRequestDto,
        GenerationEstimateRequestDto,
        GenerationAnlasEstimateDto,
        RunGenerationJobRequestDto,
        QueueDelayDto,
        QueueDirectiveDto,
        GenerationStatusDto,
        GenerationStatusQueryDto,
    )
}

fn export_history_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        RunHistoryKindDto,
        RunHistoryStatusDto,
        RunHistoryQueryDto,
        RunHistoryOutputDto,
        RunHistoryItemDto,
        RunHistoryPageDto,
        DeleteRunHistoryItemsRequestDto,
        DeleteRunHistoryItemsResponseDto,
        RerunGenerationHistoryItemRequestDto,
        RerunGenerationHistoryItemResponseDto,
    )
}

fn export_pagination_types(config: &Config) -> Result<(), String> {
    export_types!(config, PageQueryDto, PageInfoDto)
}

fn export_prompt_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        PromptChunkDto,
        UpsertPromptChunkRequestDto,
        GetPromptChunkRequestDto,
        ListPromptChunksRequestDto,
        PromptChunkPageDto,
        DeletePromptChunkRequestDto,
        DeletePromptChunkResponseDto,
        PromptPresetKindDto,
        PromptPresetDto,
        UpsertPromptPresetRequestDto,
        ListPromptPresetsRequestDto,
        PromptPresetPageDto,
        DeletePromptPresetRequestDto,
        DeletePromptPresetResponseDto,
        CompilePromptRequestDto,
        CompiledPromptDto,
        CompileGenerationCharacterPromptDto,
        CompileGenerationPromptRequestDto,
        CompiledGenerationCharacterPromptDto,
        CompiledGenerationPromptDto,
        PromptTraceDto,
        PromptFunctionTraceEntryDto,
        PromptLexiconCatalogDto,
        PromptLexiconStatsDto,
        PromptLexiconCategorySummaryDto,
        PromptLexiconSubcategorySummaryDto,
        PromptLexiconListQueryDto,
        PromptLexiconSearchQueryDto,
        PromptLexiconPageDto,
        PromptLexiconEntryDto,
    )
}

fn export_resource_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        ResourceRefDto,
        ImageInputDto,
        ImageResourceKindDto,
        ImportImageResourceRequestDto,
        ImportImageResourceResponseDto,
        GetResourceImageRequestDto,
        SaveResourceImageRequestDto,
        ResourceImageDto,
    )
}

fn export_settings_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        WorkspaceSettingsDto,
        GenerationDefaultsDto,
        ImageVariantSettingsDto,
        UpdateWorkspaceSettingsRequestDto,
        ResetWorkspaceSettingsResponseDto,
    )
}

fn export_vibe_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        VibeModelDto,
        VibeExportFormatDto,
        ImportVibeDocumentRequestDto,
        ImportEmbeddedPngVibeDocumentRequestDto,
        ExportVibeDocumentRequestDto,
        ListVibeDocumentsRequestDto,
        GetVibeDocumentRequestDto,
        RenameVibeDocumentRequestDto,
        SetVibeDocumentHiddenRequestDto,
        EnsureVibeEncodingRequestDto,
        VibeEncodingConfigDto,
        VibeDocumentEntryDto,
        VibeDocumentPageDto,
        ImportedVibeDocumentsDto,
        ExportedVibeDocumentDto,
        EnsuredVibeEncodingDto,
    )
}

fn export_workspace_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        OpenWorkspaceRequestDto,
        WorkspaceStatusDto,
        CloseWorkspaceResponseDto,
    )
}

fn export_type<T: TS + 'static>(config: &Config) -> Result<(), String> {
    T::export_all(config).map_err(|error| error.to_string())
}

fn prepare_output_dir(out_dir: &Path) -> Result<(), String> {
    if out_dir.exists() {
        validate_generated_output_dir(out_dir)?;
        fs::remove_dir_all(out_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(out_dir).map_err(|error| error.to_string())?;
    Ok(())
}

fn validate_generated_output_dir(out_dir: &Path) -> Result<(), String> {
    let is_generated_types_dir = out_dir.file_name().and_then(|name| name.to_str())
        == Some("generated")
        && out_dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("types");

    if is_generated_types_dir {
        Ok(())
    } else {
        Err(format!(
            "refusing to recursively clear non-generated type directory {}",
            out_dir.display()
        ))
    }
}

fn write_index_file(out_dir: &Path) -> Result<(), String> {
    let mut exports = fs::read_dir(out_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file()
                || path.extension().and_then(|extension| extension.to_str()) != Some("ts")
            {
                return None;
            }
            let name = path.file_stem()?.to_str()?;
            if name == "index" {
                return None;
            }
            Some(format!("export type {{ {name} }} from './{name}';"))
        })
        .collect::<Vec<_>>();
    exports.sort();
    fs::write(out_dir.join("index.ts"), exports.join("\n") + "\n")
        .map_err(|error| error.to_string())
}
