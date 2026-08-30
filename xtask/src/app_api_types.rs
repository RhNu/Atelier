use std::fs;
use std::path::{Path, PathBuf};

use atelier_app_api::{
    account::{
        ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
        ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, SubscriptionSummaryDto,
        UpdateApiKeyRequestDto, V5UsageStatusDto,
    },
    danbooru::{
        DanbooruAccountDto, DanbooruAccountStateDto, DanbooruAuthModeDto, DanbooruMediaRequestDto,
        DanbooruMediaVariantDto, DanbooruPostDetailDto, DanbooruPostDetailRequestDto,
        DanbooruPostPageDto, DanbooruPostSummaryDto, DanbooruRatingDto, DanbooruSearchRequestDto,
        DanbooruTagCategoryDto, DanbooruTagDto, SaveDanbooruAccountRequestDto,
    },
    director::{DirectorToolDto, DirectorToolResultDto, RunDirectorToolRequestDto},
    downloadable_resource::{
        DownloadableResourceGroupDto, DownloadableResourceGroupRequestDto,
        DownloadableResourceInstallProgressDto, DownloadableResourceRequestDto,
        DownloadableResourceStateDto, DownloadableResourceStatusDto, DownloadableResourcesDto,
    },
    error::ErrorEnvelopeDto,
    event::{AppEventDto, AppEventKindDto, AppEventPageDto, EventsSinceRequestDto},
    gallery::{
        DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryImageReferenceDto,
        GalleryImageReferenceRequestDto, GalleryImageReferenceTargetDto, GalleryItemDetailDto,
        GalleryItemDetailRequestDto, GalleryItemDto, GalleryMetadataStatusDto,
        GalleryMetadataWarningCodeDto, GalleryMetadataWarningDto, GalleryPageDto, GalleryQueryDto,
        GallerySafetyDto, GallerySafetyLabelDto, GallerySafetyModelEvidenceDto,
        GallerySafetyOverrideDto, GallerySafetyRatingScoresDto, GallerySafetyReviewDto,
        GallerySafetyReviewStateDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
        GallerySourceKindDto, RescanGallerySafetyRequestDto, RescanGallerySafetyResponseDto,
        SetGallerySafetyOverrideRequestDto, VisualAssetDto,
    },
    generation::{
        AnlasEstimateStatusDto, CharacterDto, CharacterPositionDto, CharacterReferenceDto,
        CharacterReferenceTypeDto, GenerateImageRequestDto, GenerateImageStreamRequestDto,
        GenerationAnlasEstimateDto, GenerationDraftCharacterDto,
        GenerationDraftCharacterPositionModeDto, GenerationDraftDto, GenerationDraftI2iDto,
        GenerationDraftPreciseReferenceDto, GenerationDraftPromptStateDto,
        GenerationDraftSeedModeDto, GenerationDraftVibeDto, GenerationDraftVibeSlotDto,
        GenerationEstimateRequestDto, GenerationPlanContextDto, GenerationRequestStatusDto,
        GenerationStatusDto, GenerationStatusQueryDto, GenerationWorkRequestDto, ImageFormatDto,
        ImageModelDescriptorDto, ImageModelDto, ImageSizeDto, Img2ImgRequestDto,
        ModelCapabilitiesDto, NoiseScheduleDto, PromptStructureDto, QualityPresetDto,
        QueueDelayDto, QueueDirectiveDto, RunGenerationJobRequestDto, SamplerDto,
        SaveGenerationDraftRequestDto, StreamModeDto, SubmitGenerationBatchJobDto,
        SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto, UcPresetDto, VibeReferenceDto,
        VibeTransferConfigDto,
    },
    history::{
        DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
        DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
        GenerationBatchHistoryStatusDto, GenerationHistoryBatchDetailDto,
        GenerationHistoryBatchDto, GenerationHistoryBatchRequestDto, GenerationHistoryPageDto,
        GenerationHistoryQueryDto, GenerationHistoryRequestDto,
        RerunGenerationHistoryBatchRequestDto, RerunGenerationHistoryBatchResponseDto,
        RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto,
        RunHistoryItemDto, RunHistoryKindDto, RunHistoryOutputDto, RunHistoryPageDto,
        RunHistoryQueryDto, RunHistoryStatusDto,
    },
    pagination::{PageInfoDto, PageQueryDto},
    prompt::{
        AppendLexiconEntitiesRequestDto, CompileGenerationCharacterPromptDto,
        CompileGenerationPromptRequestDto, CompilePromptRequestDto,
        CompiledGenerationCharacterPromptDto, CompiledGenerationPromptDto, CompiledPromptDto,
        DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, DeletePromptPresetRequestDto,
        DeletePromptPresetResponseDto, GetPromptChunkRequestDto, LexiconBootstrapDto,
        LexiconCapabilityStatusDto, LexiconCategoryDto, LexiconCompleteRequestDto,
        LexiconContentRatingDto, LexiconDraftTargetDto, LexiconEntityDetailDto,
        LexiconEntityKindDto, LexiconEntityRequestDto, LexiconFacetDto, LexiconGroupSummaryDto,
        LexiconRelatedEntityDto, LexiconSearchFiltersDto, LexiconSearchItemDto,
        LexiconSearchModeDto, LexiconSearchPageDto, LexiconSearchRequestDto, LexiconStatsDto,
        ListPromptChunksRequestDto, ListPromptPresetsRequestDto, LocalizedLexiconTextDto,
        PromptChunkDto, PromptChunkPageDto, PromptFunctionTraceEntryDto, PromptPresetDto,
        PromptPresetKindDto, PromptPresetPageDto, PromptTraceDto, UpsertPromptChunkRequestDto,
        UpsertPromptPresetRequestDto,
    },
    resource::{
        CopyResourceImageRequestDto, GetResourceImageRequestDto, ImageExportFormatDto,
        ImageInputDto, ImageResourceKindDto, ImportImageResourceRequestDto,
        ImportImageResourceResponseDto, ReleaseImportedImageResourcesRequestDto,
        ReleaseImportedImageResourcesResponseDto, ResourceImageDto, ResourceRefDto,
        SaveResourceImageRequestDto, SaveResourceImagesZipEntryDto,
        SaveResourceImagesZipRequestDto,
    },
    settings::{
        GenerationDefaultsDto, GlobalFrontendSettingsDto, GlobalGallerySettingsDto,
        GlobalSafetySettingsDto, GlobalSettingsDto, ImageVariantSettingsDto,
        ResetWorkspaceSettingsResponseDto, UpdateGlobalSettingsRequestDto,
        UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
    },
    vibe::{
        EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
        ExportedVibeDocumentDto, GetVibeDocumentRequestDto,
        ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
        ImportedVibeDocumentsDto, ListVibeDocumentsRequestDto, RenameVibeDocumentRequestDto,
        SetVibeDocumentHiddenRequestDto, VibeDocumentEntryDto, VibeDocumentPageDto,
        VibeEncodingConfigDto, VibeExportFormatDto,
    },
    workspace::{
        AppBootstrapDto, CloseWorkspaceResponseDto, OpenWorkspaceRequestDto,
        WorkspaceRestoreFailureDto, WorkspaceStatusDto,
    },
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
    export_danbooru_types(&ts_config)?;
    export_event_types(&ts_config)?;
    export_gallery_types(&ts_config)?;
    export_generation_types(&ts_config)?;
    export_history_types(&ts_config)?;
    export_downloadable_resource_types(&ts_config)?;
    export_pagination_types(&ts_config)?;
    export_prompt_types(&ts_config)?;
    export_resource_types(&ts_config)?;
    export_settings_types(&ts_config)?;
    export_vibe_types(&ts_config)?;
    export_workspace_types(&ts_config)?;

    write_index_file(&config.out_dir)?;
    Ok(())
}

fn export_danbooru_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        DanbooruAccountStateDto,
        DanbooruAccountDto,
        SaveDanbooruAccountRequestDto,
        DanbooruRatingDto,
        DanbooruAuthModeDto,
        DanbooruTagCategoryDto,
        DanbooruMediaVariantDto,
        DanbooruSearchRequestDto,
        DanbooruPostSummaryDto,
        DanbooruPostPageDto,
        DanbooruPostDetailRequestDto,
        DanbooruTagDto,
        DanbooruPostDetailDto,
        DanbooruMediaRequestDto,
    )
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
        V5UsageStatusDto,
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
        GalleryItemDetailRequestDto,
        GalleryItemDetailDto,
        GalleryMetadataStatusDto,
        GalleryMetadataWarningCodeDto,
        GalleryMetadataWarningDto,
        GallerySafetyScanStateDto,
        GallerySafetyRiskBandDto,
        GallerySafetyLabelDto,
        GallerySafetyRatingScoresDto,
        GallerySafetyModelEvidenceDto,
        GallerySafetyReviewStateDto,
        GallerySafetyReviewDto,
        GallerySafetyDto,
        VisualAssetDto,
        DeleteGalleryItemsRequestDto,
        DeleteGalleryItemsResponseDto,
        RescanGallerySafetyRequestDto,
        RescanGallerySafetyResponseDto,
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
        ImageModelDescriptorDto,
        ModelCapabilitiesDto,
        PromptStructureDto,
        ImageSizeDto,
        SamplerDto,
        NoiseScheduleDto,
        UcPresetDto,
        ImageFormatDto,
        StreamModeDto,
        QualityPresetDto,
        Img2ImgRequestDto,
        VibeReferenceDto,
        VibeTransferConfigDto,
        CharacterReferenceTypeDto,
        CharacterReferenceDto,
        CharacterPositionDto,
        CharacterDto,
        GenerationDraftSeedModeDto,
        GenerationDraftCharacterPositionModeDto,
        GenerationDraftI2iDto,
        GenerationDraftVibeSlotDto,
        GenerationDraftVibeDto,
        GenerationDraftPreciseReferenceDto,
        GenerationDraftCharacterDto,
        GenerationDraftPromptStateDto,
        GenerationDraftDto,
        SaveGenerationDraftRequestDto,
        GenerateImageRequestDto,
        GenerateImageStreamRequestDto,
        GenerationWorkRequestDto,
        GenerationPlanContextDto,
        SubmitGenerationRequestDto,
        SubmitGenerationBatchJobDto,
        SubmitGenerationBatchRequestDto,
        GenerationEstimateRequestDto,
        AnlasEstimateStatusDto,
        GenerationAnlasEstimateDto,
        RunGenerationJobRequestDto,
        QueueDelayDto,
        QueueDirectiveDto,
        GenerationRequestStatusDto,
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
        GenerationBatchHistoryStatusDto,
        GenerationHistoryQueryDto,
        GenerationHistoryBatchDto,
        GenerationHistoryPageDto,
        GenerationHistoryBatchRequestDto,
        GenerationHistoryRequestDto,
        GenerationHistoryBatchDetailDto,
        DeleteGenerationHistoryBatchesRequestDto,
        DeleteGenerationHistoryBatchesResponseDto,
        RerunGenerationHistoryBatchRequestDto,
        RerunGenerationHistoryBatchResponseDto,
    )
}

fn export_downloadable_resource_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        DownloadableResourceStateDto,
        DownloadableResourceStatusDto,
        DownloadableResourceGroupDto,
        DownloadableResourcesDto,
        DownloadableResourceRequestDto,
        DownloadableResourceGroupRequestDto,
        DownloadableResourceInstallProgressDto,
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
        LexiconEntityKindDto,
        LexiconCategoryDto,
        LexiconContentRatingDto,
        LexiconSearchModeDto,
        LexiconDraftTargetDto,
        LexiconCapabilityStatusDto,
        LexiconStatsDto,
        LexiconFacetDto,
        LexiconGroupSummaryDto,
        LexiconBootstrapDto,
        LexiconSearchItemDto,
        LexiconCompleteRequestDto,
        LexiconSearchFiltersDto,
        LexiconSearchRequestDto,
        LexiconSearchPageDto,
        LexiconEntityRequestDto,
        LocalizedLexiconTextDto,
        LexiconRelatedEntityDto,
        LexiconEntityDetailDto,
        AppendLexiconEntitiesRequestDto,
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
        ReleaseImportedImageResourcesRequestDto,
        ReleaseImportedImageResourcesResponseDto,
        GetResourceImageRequestDto,
        ImageExportFormatDto,
        SaveResourceImageRequestDto,
        CopyResourceImageRequestDto,
        SaveResourceImagesZipEntryDto,
        SaveResourceImagesZipRequestDto,
        ResourceImageDto,
    )
}

fn export_settings_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
        WorkspaceSettingsDto,
        GenerationDefaultsDto,
        ImageVariantSettingsDto,
        GlobalSettingsDto,
        GlobalFrontendSettingsDto,
        GlobalGallerySettingsDto,
        GlobalSafetySettingsDto,
        UpdateGlobalSettingsRequestDto,
        UpdateWorkspaceSettingsRequestDto,
        ResetWorkspaceSettingsResponseDto,
    )
}

fn export_vibe_types(config: &Config) -> Result<(), String> {
    export_types!(
        config,
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
        WorkspaceRestoreFailureDto,
        AppBootstrapDto,
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
