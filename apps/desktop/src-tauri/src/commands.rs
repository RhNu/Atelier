mod app_update;
mod desktop_io;
mod resource_image;

pub use app_update::*;
pub use desktop_io::*;

use std::sync::Arc;

use atelier_app::CommandResult;
use atelier_app_api::{
    account::{
        ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
        ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, SubscriptionSummaryDto,
        UpdateApiKeyRequestDto,
    },
    danbooru::{DanbooruAccountDto, SaveDanbooruAccountRequestDto},
    director::{DirectorToolResultDto, RunDirectorToolRequestDto},
    downloadable_resource::{
        DownloadableResourceGroupRequestDto, DownloadableResourceInstallProgressDto,
        DownloadableResourceRequestDto, DownloadableResourceStatusDto, DownloadableResourcesDto,
    },
    error::ErrorEnvelopeDto,
    event::{AppEventPageDto, EventsSinceRequestDto},
    explore::{
        ExploreItemRefDto, ExploreMediaRequestDto, ExplorePageDto, ExplorePostDetailDto,
        ExploreSearchRequestDto, ExploreSourceDescriptorDto,
    },
    gallery::{
        DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryImageReferenceDto,
        GalleryImageReferenceRequestDto, GalleryItemDetailDto, GalleryItemDetailRequestDto,
        GalleryItemDto, GalleryPageDto, GalleryQueryDto, RescanGallerySafetyRequestDto,
        RescanGallerySafetyResponseDto, SetGallerySafetyOverrideRequestDto,
    },
    generation::{
        GenerationAnlasEstimateDto, GenerationDraftDto, GenerationEstimateRequestDto,
        GenerationStatusDto, GenerationStatusQueryDto, QueueDirectiveDto,
        RunGenerationJobRequestDto, SaveGenerationDraftRequestDto, SubmitGenerationBatchRequestDto,
        SubmitGenerationRequestDto,
    },
    history::{
        DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
        DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
        GenerationHistoryBatchDetailDto, GenerationHistoryBatchRequestDto,
        GenerationHistoryPageDto, GenerationHistoryQueryDto, RerunGenerationHistoryBatchRequestDto,
        RerunGenerationHistoryBatchResponseDto, RerunGenerationHistoryItemRequestDto,
        RerunGenerationHistoryItemResponseDto, RunHistoryPageDto, RunHistoryQueryDto,
    },
    prompt::{
        AppendLexiconEntitiesRequestDto, CompileGenerationPromptRequestDto,
        CompilePromptRequestDto, CompiledGenerationPromptDto, CompiledPromptDto,
        DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, DeletePromptPresetRequestDto,
        DeletePromptPresetResponseDto, GetPromptChunkRequestDto, LexiconBootstrapDto,
        LexiconCompleteRequestDto, LexiconEntityDetailDto, LexiconEntityRequestDto,
        LexiconSearchItemDto, LexiconSearchPageDto, LexiconSearchRequestDto,
        ListPromptChunksRequestDto, ListPromptPresetsRequestDto, PromptChunkDto,
        PromptChunkPageDto, PromptPresetDto, PromptPresetPageDto, UpsertPromptChunkRequestDto,
        UpsertPromptPresetRequestDto,
    },
    resource::{
        GetResourceImageRequestDto, ReleaseImportedImageResourcesRequestDto,
        ReleaseImportedImageResourcesResponseDto, ResourceImageDto,
    },
    settings::{
        FrontendLanguageDto, GlobalSettingsDto, ResetWorkspaceSettingsResponseDto,
        UpdateGlobalSettingsRequestDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
    },
    vibe::{
        EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, GetVibeDocumentRequestDto,
        ListVibeDocumentsRequestDto, RenameVibeDocumentRequestDto, SetVibeDocumentHiddenRequestDto,
        VibeDocumentEntryDto, VibeDocumentPageDto,
    },
};
use atelier_downloadable_resources::{ResourceInstallProgress, ResourceInstallProgressSink};
use tauri::{State, ipc::Channel};

use crate::desktop::DesktopState;

fn join_error(error: impl std::fmt::Display) -> ErrorEnvelopeDto {
    ErrorEnvelopeDto::new(
        "desktop_background_task",
        format!("desktop background task failed: {error}"),
    )
}

struct ResourceProgressChannel(Channel<DownloadableResourceInstallProgressDto>);

impl ResourceInstallProgressSink for ResourceProgressChannel {
    fn report(&self, progress: ResourceInstallProgress) {
        if let Err(error) = self.0.send(DownloadableResourceInstallProgressDto {
            resource_id: progress.resource_id,
            downloaded_bytes: progress.downloaded_bytes,
            total_bytes: progress.total_bytes,
        }) {
            log::debug!("resource install progress channel closed: {error}");
        }
    }
}

#[tauri::command]
pub async fn get_global_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<GlobalSettingsDto> {
    state.host.get_global_settings().await
}

#[tauri::command]
pub async fn update_global_settings(
    state: State<'_, DesktopState>,
    request: UpdateGlobalSettingsRequestDto,
) -> CommandResult<GlobalSettingsDto> {
    let settings = state.host.update_global_settings(request).await?;
    state.set_notification_language(settings.frontend.language);
    Ok(settings)
}

// Tauri injects managed `State` command arguments by value.
#[allow(clippy::needless_pass_by_value)]
#[tauri::command]
pub fn set_notification_language(state: State<'_, DesktopState>, language: FrontendLanguageDto) {
    state.set_notification_language(language);
}

#[tauri::command]
pub async fn get_danbooru_account(
    state: State<'_, DesktopState>,
) -> CommandResult<DanbooruAccountDto> {
    state.host.get_danbooru_account().await
}

#[tauri::command]
pub async fn save_danbooru_account(
    state: State<'_, DesktopState>,
    request: SaveDanbooruAccountRequestDto,
) -> CommandResult<DanbooruAccountDto> {
    state.host.save_danbooru_account(request).await
}

#[tauri::command]
pub async fn probe_danbooru_account(
    state: State<'_, DesktopState>,
) -> CommandResult<DanbooruAccountDto> {
    state.host.probe_danbooru_account().await
}

#[tauri::command]
pub async fn delete_danbooru_account(
    state: State<'_, DesktopState>,
) -> CommandResult<DanbooruAccountDto> {
    state.host.delete_danbooru_account().await
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri extracts State by value.
pub fn list_explore_sources(state: State<'_, DesktopState>) -> Vec<ExploreSourceDescriptorDto> {
    state.host.list_explore_sources()
}

#[tauri::command]
pub async fn search_explore_posts(
    state: State<'_, DesktopState>,
    request: ExploreSearchRequestDto,
) -> CommandResult<ExplorePageDto> {
    state.host.search_explore_posts(request).await
}

#[tauri::command]
pub async fn get_explore_post_detail(
    state: State<'_, DesktopState>,
    item: ExploreItemRefDto,
) -> CommandResult<ExplorePostDetailDto> {
    state.host.get_explore_post_detail(item).await
}

#[tauri::command]
pub async fn get_explore_media(
    state: State<'_, DesktopState>,
    request: ExploreMediaRequestDto,
) -> CommandResult<ResourceImageDto> {
    state.host.get_explore_media(request).await
}

#[tauri::command]
pub async fn list_downloadable_resources(
    state: State<'_, DesktopState>,
) -> CommandResult<DownloadableResourcesDto> {
    state.host.list_downloadable_resources().await
}

#[tauri::command]
pub async fn refresh_downloadable_resource_catalog(
    state: State<'_, DesktopState>,
) -> CommandResult<DownloadableResourcesDto> {
    state.host.refresh_downloadable_resource_catalog().await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri State command boundary"
)]
pub fn complete_downloadable_resource_onboarding(
    state: State<'_, DesktopState>,
) -> CommandResult<()> {
    state.host.complete_downloadable_resource_onboarding()
}

#[tauri::command]
pub async fn install_downloadable_resource(
    state: State<'_, DesktopState>,
    request: DownloadableResourceRequestDto,
    on_progress: Channel<DownloadableResourceInstallProgressDto>,
) -> CommandResult<DownloadableResourceStatusDto> {
    let should_rescan = request.resource_id == "anime-dbrating";
    let status = state
        .host
        .install_downloadable_resource(request, Some(&ResourceProgressChannel(on_progress)))
        .await?;
    if should_rescan {
        let host = state.host.clone();
        tauri::async_runtime::spawn(async move {
            let _ = host
                .rescan_gallery_safety(RescanGallerySafetyRequestDto::default())
                .await;
        });
    }
    Ok(status)
}

#[tauri::command]
pub async fn install_downloadable_resource_group(
    state: State<'_, DesktopState>,
    request: DownloadableResourceGroupRequestDto,
    on_progress: Channel<DownloadableResourceInstallProgressDto>,
) -> CommandResult<Vec<DownloadableResourceStatusDto>> {
    state
        .host
        .install_downloadable_resource_group(request, Some(&ResourceProgressChannel(on_progress)))
        .await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri State command boundary"
)]
pub fn cancel_downloadable_resource_install(
    state: State<'_, DesktopState>,
    request: DownloadableResourceRequestDto,
) -> CommandResult<()> {
    state.host.cancel_downloadable_resource_install(request)
}

#[tauri::command]
pub async fn delete_downloadable_resource(
    state: State<'_, DesktopState>,
    request: DownloadableResourceRequestDto,
) -> CommandResult<()> {
    state.host.delete_downloadable_resource(request).await
}

#[tauri::command]
pub async fn create_api_key(
    state: State<'_, DesktopState>,
    request: CreateApiKeyRequestDto,
) -> CommandResult<ApiKeyRecordDto> {
    state.host.create_api_key(request).await
}

#[tauri::command]
pub async fn update_api_key(
    state: State<'_, DesktopState>,
    request: UpdateApiKeyRequestDto,
) -> CommandResult<ApiKeyRecordDto> {
    state.host.update_api_key(request).await
}

#[tauri::command]
pub async fn delete_api_key(
    state: State<'_, DesktopState>,
    request: DeleteApiKeyRequestDto,
) -> CommandResult<DeleteApiKeyResponseDto> {
    state.host.delete_api_key(request).await
}

#[tauri::command]
pub async fn list_api_keys(state: State<'_, DesktopState>) -> CommandResult<Vec<ApiKeyRecordDto>> {
    state.host.list_api_keys().await
}

#[tauri::command]
pub async fn set_active_api_key(
    state: State<'_, DesktopState>,
    request: SetActiveApiKeyRequestDto,
) -> CommandResult<()> {
    state.host.set_active_api_key(request).await
}

#[tauri::command]
pub async fn probe_api_key(
    state: State<'_, DesktopState>,
    request: ProbeApiKeyRequestDto,
) -> CommandResult<SubscriptionSummaryDto> {
    state.host.probe_api_key(request).await
}

#[tauri::command]
pub async fn probe_active_api_key(
    state: State<'_, DesktopState>,
) -> CommandResult<SubscriptionSummaryDto> {
    state.host.probe_active_api_key().await
}

#[tauri::command]
pub async fn upsert_prompt_chunk(
    state: State<'_, DesktopState>,
    request: UpsertPromptChunkRequestDto,
) -> CommandResult<PromptChunkDto> {
    state.host.upsert_prompt_chunk(request).await
}

#[tauri::command]
pub async fn get_prompt_chunk(
    state: State<'_, DesktopState>,
    request: GetPromptChunkRequestDto,
) -> CommandResult<PromptChunkDto> {
    state.host.get_prompt_chunk(request).await
}

#[tauri::command]
pub async fn list_prompt_chunks(
    state: State<'_, DesktopState>,
    request: ListPromptChunksRequestDto,
) -> CommandResult<PromptChunkPageDto> {
    state.host.list_prompt_chunks(request).await
}

#[tauri::command]
pub async fn delete_prompt_chunk(
    state: State<'_, DesktopState>,
    request: DeletePromptChunkRequestDto,
) -> CommandResult<DeletePromptChunkResponseDto> {
    state.host.delete_prompt_chunk(request).await
}

#[tauri::command]
pub async fn upsert_prompt_preset(
    state: State<'_, DesktopState>,
    request: UpsertPromptPresetRequestDto,
) -> CommandResult<PromptPresetDto> {
    state.host.upsert_prompt_preset(request).await
}

#[tauri::command]
pub async fn list_prompt_presets(
    state: State<'_, DesktopState>,
    request: ListPromptPresetsRequestDto,
) -> CommandResult<PromptPresetPageDto> {
    state.host.list_prompt_presets(request).await
}

#[tauri::command]
pub async fn delete_prompt_preset(
    state: State<'_, DesktopState>,
    request: DeletePromptPresetRequestDto,
) -> CommandResult<DeletePromptPresetResponseDto> {
    state.host.delete_prompt_preset(request).await
}

#[tauri::command]
pub async fn compile_prompt_preview(
    state: State<'_, DesktopState>,
    request: CompilePromptRequestDto,
) -> CommandResult<CompiledPromptDto> {
    state.host.compile_prompt_preview(request).await
}

#[tauri::command]
pub async fn compile_generation_prompt_preview(
    state: State<'_, DesktopState>,
    request: CompileGenerationPromptRequestDto,
) -> CommandResult<CompiledGenerationPromptDto> {
    state.host.compile_generation_prompt_preview(request).await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub async fn lexicon_bootstrap(
    state: State<'_, DesktopState>,
) -> CommandResult<LexiconBootstrapDto> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_bootstrap())
        .await
        .map_err(join_error)?
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub async fn lexicon_complete(
    state: State<'_, DesktopState>,
    request: LexiconCompleteRequestDto,
) -> CommandResult<Vec<LexiconSearchItemDto>> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_complete(&request))
        .await
        .map_err(join_error)?
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub async fn lexicon_search(
    state: State<'_, DesktopState>,
    request: LexiconSearchRequestDto,
) -> CommandResult<LexiconSearchPageDto> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_search(request))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn lexicon_entity(
    state: State<'_, DesktopState>,
    request: LexiconEntityRequestDto,
) -> CommandResult<LexiconEntityDetailDto> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_entity(&request))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn get_resource_image(
    state: State<'_, DesktopState>,
    request: GetResourceImageRequestDto,
) -> CommandResult<ResourceImageDto> {
    state.host.get_resource_image(request).await
}

#[tauri::command]
pub async fn release_imported_image_resources(
    state: State<'_, DesktopState>,
    request: ReleaseImportedImageResourcesRequestDto,
) -> CommandResult<ReleaseImportedImageResourcesResponseDto> {
    state.host.release_imported_image_resources(request).await
}

#[tauri::command]
pub async fn get_workspace_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<WorkspaceSettingsDto> {
    state.host.get_workspace_settings().await
}

#[tauri::command]
pub async fn update_workspace_settings(
    state: State<'_, DesktopState>,
    request: UpdateWorkspaceSettingsRequestDto,
) -> CommandResult<WorkspaceSettingsDto> {
    state.host.update_workspace_settings(request).await
}

#[tauri::command]
pub async fn reset_workspace_settings(
    state: State<'_, DesktopState>,
) -> CommandResult<ResetWorkspaceSettingsResponseDto> {
    state.host.reset_workspace_settings().await
}

#[tauri::command]
pub async fn get_generation_draft(
    state: State<'_, DesktopState>,
) -> CommandResult<Option<GenerationDraftDto>> {
    state.host.get_generation_draft().await
}

#[tauri::command]
pub async fn save_generation_draft(
    state: State<'_, DesktopState>,
    request: SaveGenerationDraftRequestDto,
) -> CommandResult<GenerationDraftDto> {
    state.host.save_generation_draft(request).await
}

#[tauri::command]
pub async fn clear_generation_draft(state: State<'_, DesktopState>) -> CommandResult<()> {
    state.host.clear_generation_draft().await
}

#[tauri::command]
pub async fn append_lexicon_entities_to_generation_draft(
    state: State<'_, DesktopState>,
    request: AppendLexiconEntitiesRequestDto,
) -> CommandResult<GenerationDraftDto> {
    state
        .host
        .append_lexicon_entities_to_generation_draft(request)
        .await
}

#[tauri::command]
pub async fn submit_generation(
    state: State<'_, DesktopState>,
    request: SubmitGenerationRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.submit_generation(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn submit_generation_batch(
    state: State<'_, DesktopState>,
    request: SubmitGenerationBatchRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.submit_generation_batch(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn estimate_generation(
    state: State<'_, DesktopState>,
    request: GenerationEstimateRequestDto,
) -> CommandResult<GenerationAnlasEstimateDto> {
    state.host.estimate_generation(request).await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands extract managed state by value"
)]
pub fn list_image_models(
    state: State<'_, DesktopState>,
) -> CommandResult<Vec<atelier_app_api::generation::ImageModelDescriptorDto>> {
    state.host.list_image_models()
}

#[tauri::command]
pub async fn run_generation_job(
    state: State<'_, DesktopState>,
    request: RunGenerationJobRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.run_generation_job(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn pause_generation_queue(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    state.cancel_generation_worker();
    state.host.pause_generation_queue().await
}

#[tauri::command]
pub async fn resume_generation_queue(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.resume_generation_queue().await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn stop_generation_queue(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    state.cancel_generation_worker_and_clear_pending();
    state.host.stop_generation_queue().await
}

#[tauri::command]
pub async fn generation_delay_elapsed(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.generation_delay_elapsed().await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn generation_status(
    state: State<'_, DesktopState>,
    request: GenerationStatusQueryDto,
) -> CommandResult<GenerationStatusDto> {
    state.host.generation_status(request).await
}

#[tauri::command]
pub async fn query_run_history(
    state: State<'_, DesktopState>,
    request: RunHistoryQueryDto,
) -> CommandResult<RunHistoryPageDto> {
    state.host.query_run_history(request).await
}

#[tauri::command]
pub async fn query_generation_history(
    state: State<'_, DesktopState>,
    request: GenerationHistoryQueryDto,
) -> CommandResult<GenerationHistoryPageDto> {
    state.host.query_generation_history(request).await
}

#[tauri::command]
pub async fn get_generation_history_batch(
    state: State<'_, DesktopState>,
    request: GenerationHistoryBatchRequestDto,
) -> CommandResult<GenerationHistoryBatchDetailDto> {
    state.host.get_generation_history_batch(request).await
}

#[tauri::command]
pub async fn delete_run_history_items(
    state: State<'_, DesktopState>,
    request: DeleteRunHistoryItemsRequestDto,
) -> CommandResult<DeleteRunHistoryItemsResponseDto> {
    state.host.delete_run_history_items(request).await
}

#[tauri::command]
pub async fn delete_generation_history_batches(
    state: State<'_, DesktopState>,
    request: DeleteGenerationHistoryBatchesRequestDto,
) -> CommandResult<DeleteGenerationHistoryBatchesResponseDto> {
    state.host.delete_generation_history_batches(request).await
}

#[tauri::command]
pub async fn rerun_generation_history_item(
    state: State<'_, DesktopState>,
    request: RerunGenerationHistoryItemRequestDto,
) -> CommandResult<RerunGenerationHistoryItemResponseDto> {
    let response = state.host.rerun_generation_history_item(request).await?;
    state.kick_generation_worker(response.directive.clone());
    Ok(response)
}

#[tauri::command]
pub async fn rerun_generation_history_batch(
    state: State<'_, DesktopState>,
    request: RerunGenerationHistoryBatchRequestDto,
) -> CommandResult<RerunGenerationHistoryBatchResponseDto> {
    let response = state.host.rerun_generation_history_batch(request).await?;
    state.kick_generation_worker(response.directive.clone());
    Ok(response)
}

#[tauri::command]
pub async fn run_director_tool(
    state: State<'_, DesktopState>,
    request: RunDirectorToolRequestDto,
) -> CommandResult<DirectorToolResultDto> {
    state.host.run_director_tool(request).await
}

#[tauri::command]
pub async fn ensure_vibe_encoding(
    state: State<'_, DesktopState>,
    request: EnsureVibeEncodingRequestDto,
) -> CommandResult<EnsuredVibeEncodingDto> {
    state.host.ensure_vibe_encoding(request).await
}

#[tauri::command]
pub async fn list_vibe_documents(
    state: State<'_, DesktopState>,
    request: ListVibeDocumentsRequestDto,
) -> CommandResult<VibeDocumentPageDto> {
    state.host.list_vibe_documents(request).await
}

#[tauri::command]
pub async fn get_vibe_document(
    state: State<'_, DesktopState>,
    request: GetVibeDocumentRequestDto,
) -> CommandResult<VibeDocumentEntryDto> {
    state.host.get_vibe_document(request).await
}

#[tauri::command]
pub async fn rename_vibe_document(
    state: State<'_, DesktopState>,
    request: RenameVibeDocumentRequestDto,
) -> CommandResult<VibeDocumentEntryDto> {
    state.host.rename_vibe_document(request).await
}

#[tauri::command]
pub async fn set_vibe_document_hidden(
    state: State<'_, DesktopState>,
    request: SetVibeDocumentHiddenRequestDto,
) -> CommandResult<VibeDocumentEntryDto> {
    state.host.set_vibe_document_hidden(request).await
}

#[tauri::command]
pub async fn query_gallery(
    state: State<'_, DesktopState>,
    request: GalleryQueryDto,
) -> CommandResult<GalleryPageDto> {
    state.host.query_gallery(request).await
}

#[tauri::command]
pub async fn get_gallery_item_detail(
    state: State<'_, DesktopState>,
    request: GalleryItemDetailRequestDto,
) -> CommandResult<GalleryItemDetailDto> {
    state.host.get_gallery_item_detail(request).await
}

#[tauri::command]
pub async fn set_gallery_safety_override(
    state: State<'_, DesktopState>,
    request: SetGallerySafetyOverrideRequestDto,
) -> CommandResult<GalleryItemDto> {
    state.host.set_gallery_safety_override(request).await
}

#[tauri::command]
pub async fn rescan_gallery_safety(
    state: State<'_, DesktopState>,
    request: RescanGallerySafetyRequestDto,
) -> CommandResult<RescanGallerySafetyResponseDto> {
    state.host.rescan_gallery_safety(request).await
}

#[tauri::command]
pub async fn delete_gallery_items(
    state: State<'_, DesktopState>,
    request: DeleteGalleryItemsRequestDto,
) -> CommandResult<DeleteGalleryItemsResponseDto> {
    state.host.delete_gallery_items(request).await
}

#[tauri::command]
pub async fn gallery_image_reference(
    state: State<'_, DesktopState>,
    request: GalleryImageReferenceRequestDto,
) -> CommandResult<GalleryImageReferenceDto> {
    state.host.gallery_image_reference(request).await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub fn events_since(
    state: State<'_, DesktopState>,
    request: EventsSinceRequestDto,
) -> CommandResult<AppEventPageDto> {
    state.host.events_since(request)
}
