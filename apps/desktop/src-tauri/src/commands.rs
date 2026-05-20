#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use nai_atelier_adapter_desktop_system::{DesktopPaths, PickFilesOptions};
use nai_atelier_app::CommandResult;
use nai_atelier_app_api::{
    account::{
        ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
        ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, SubscriptionSummaryDto,
        UpdateApiKeyRequestDto,
    },
    director::{DirectorToolResultDto, RunDirectorToolRequestDto},
    error::ErrorEnvelopeDto,
    event::{AppEventPageDto, EventsSinceRequestDto},
    gallery::{
        GalleryImageReferenceDto, GalleryImageReferenceRequestDto, GalleryItemDto, GalleryPageDto,
        GalleryQueryDto, SetGallerySafetyOverrideRequestDto,
    },
    generation::{
        GenerationStatusDto, GenerationStatusQueryDto, QueueDirectiveDto,
        RunGenerationJobRequestDto, SubmitGenerationRequestDto,
    },
    prompt::{
        CompilePromptRequestDto, CompiledPromptDto, DeletePromptChunkRequestDto,
        DeletePromptChunkResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
        PromptChunkDto, PromptChunkPageDto, PromptLexiconCatalogDto, PromptLexiconListQueryDto,
        PromptLexiconPageDto, PromptLexiconSearchQueryDto, UpsertPromptChunkRequestDto,
    },
    resource::{ImportImageResourceRequestDto, ImportImageResourceResponseDto},
    settings::{
        ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
    },
    vibe::{
        EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
        ExportedVibeDocumentDto, ImportEmbeddedPngVibeDocumentRequestDto,
        ImportVibeDocumentRequestDto, ImportedVibeDocumentsDto,
    },
    workspace::{CloseWorkspaceResponseDto, OpenWorkspaceRequestDto, WorkspaceStatusDto},
};
use tauri::State;

use crate::desktop::{DesktopState, TauriDialog, TauriPathOpener};

#[tauri::command]
pub fn desktop_paths(state: State<'_, DesktopState>) -> DesktopPaths {
    state.system.paths().clone()
}

#[tauri::command]
pub fn pick_workspace_directory(state: State<'_, DesktopState>) -> CommandResult<Option<PathBuf>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    desktop_result(state.system.pick_workspace_directory(&dialog))
}

#[tauri::command]
pub fn pick_export_directory(state: State<'_, DesktopState>) -> CommandResult<Option<PathBuf>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    desktop_result(state.system.pick_export_directory(&dialog))
}

#[tauri::command]
pub fn pick_image_files(
    state: State<'_, DesktopState>,
    options: PickFilesOptions,
) -> CommandResult<Vec<PathBuf>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    desktop_result(state.system.pick_image_files(&dialog, options))
}

#[tauri::command]
pub fn pick_vibe_documents(
    state: State<'_, DesktopState>,
    options: PickFilesOptions,
) -> CommandResult<Vec<PathBuf>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    desktop_result(state.system.pick_vibe_documents(&dialog, options))
}

#[tauri::command]
pub fn open_path(state: State<'_, DesktopState>, path: PathBuf) -> CommandResult<()> {
    let opener = TauriPathOpener::new(state.app_handle.clone());
    desktop_result(state.system.open_path(path, &opener))
}

#[tauri::command]
pub fn reveal_path(state: State<'_, DesktopState>, path: PathBuf) -> CommandResult<()> {
    let opener = TauriPathOpener::new(state.app_handle.clone());
    desktop_result(state.system.reveal_path(path, &opener))
}

#[tauri::command]
pub async fn open_workspace(
    state: State<'_, DesktopState>,
    request: OpenWorkspaceRequestDto,
) -> CommandResult<WorkspaceStatusDto> {
    let root = request.root.clone();
    state.cancel_generation_worker_and_clear_pending();
    let status = state.host.open_workspace(request).await?;
    desktop_result(state.system.allow_user_path(root))?;
    Ok(status)
}

#[tauri::command]
pub fn workspace_status(state: State<'_, DesktopState>) -> CommandResult<WorkspaceStatusDto> {
    state.host.workspace_status()
}

#[tauri::command]
pub fn close_workspace(state: State<'_, DesktopState>) -> CommandResult<CloseWorkspaceResponseDto> {
    state.cancel_generation_worker_and_clear_pending();
    state.host.close_workspace()
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
pub async fn compile_prompt_preview(
    state: State<'_, DesktopState>,
    request: CompilePromptRequestDto,
) -> CommandResult<CompiledPromptDto> {
    state.host.compile_prompt_preview(request).await
}

#[tauri::command]
pub fn prompt_lexicon_catalog(
    state: State<'_, DesktopState>,
) -> CommandResult<PromptLexiconCatalogDto> {
    state.host.prompt_lexicon_catalog()
}

#[tauri::command]
pub fn prompt_lexicon_list(
    state: State<'_, DesktopState>,
    request: PromptLexiconListQueryDto,
) -> CommandResult<PromptLexiconPageDto> {
    state.host.prompt_lexicon_list(request)
}

#[tauri::command]
pub fn prompt_lexicon_search(
    state: State<'_, DesktopState>,
    request: PromptLexiconSearchQueryDto,
) -> CommandResult<PromptLexiconPageDto> {
    state.host.prompt_lexicon_search(request)
}

#[tauri::command]
pub async fn import_image_resource(
    state: State<'_, DesktopState>,
    request: ImportImageResourceRequestDto,
) -> CommandResult<ImportImageResourceResponseDto> {
    state.host.import_image_resource(request).await
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
pub async fn submit_generation(
    state: State<'_, DesktopState>,
    request: SubmitGenerationRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.submit_generation(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
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
pub async fn run_director_tool(
    state: State<'_, DesktopState>,
    request: RunDirectorToolRequestDto,
) -> CommandResult<DirectorToolResultDto> {
    state.host.run_director_tool(request).await
}

#[tauri::command]
pub async fn import_vibe_document(
    state: State<'_, DesktopState>,
    request: ImportVibeDocumentRequestDto,
) -> CommandResult<ImportedVibeDocumentsDto> {
    state.host.import_vibe_document(request).await
}

#[tauri::command]
pub async fn import_embedded_png_vibe_document(
    state: State<'_, DesktopState>,
    request: ImportEmbeddedPngVibeDocumentRequestDto,
) -> CommandResult<ImportedVibeDocumentsDto> {
    state.host.import_embedded_png_vibe_document(request).await
}

#[tauri::command]
pub async fn export_vibe_document(
    state: State<'_, DesktopState>,
    request: ExportVibeDocumentRequestDto,
) -> CommandResult<ExportedVibeDocumentDto> {
    state.host.export_vibe_document(request).await
}

#[tauri::command]
pub async fn ensure_vibe_encoding(
    state: State<'_, DesktopState>,
    request: EnsureVibeEncodingRequestDto,
) -> CommandResult<EnsuredVibeEncodingDto> {
    state.host.ensure_vibe_encoding(request).await
}

#[tauri::command]
pub async fn query_gallery(
    state: State<'_, DesktopState>,
    request: GalleryQueryDto,
) -> CommandResult<GalleryPageDto> {
    state.host.query_gallery(request).await
}

#[tauri::command]
pub async fn set_gallery_safety_override(
    state: State<'_, DesktopState>,
    request: SetGallerySafetyOverrideRequestDto,
) -> CommandResult<GalleryItemDto> {
    state.host.set_gallery_safety_override(request).await
}

#[tauri::command]
pub async fn gallery_image_reference(
    state: State<'_, DesktopState>,
    request: GalleryImageReferenceRequestDto,
) -> CommandResult<GalleryImageReferenceDto> {
    state.host.gallery_image_reference(request).await
}

#[tauri::command]
pub fn events_since(
    state: State<'_, DesktopState>,
    request: EventsSinceRequestDto,
) -> CommandResult<AppEventPageDto> {
    state.host.events_since(request)
}

fn desktop_result<T>(
    result: nai_atelier_adapter_desktop_system::DesktopSystemResult<T>,
) -> CommandResult<T> {
    result.map_err(|error| ErrorEnvelopeDto::new("desktop_system_error", error.to_string()))
}
