#![allow(clippy::needless_pass_by_value)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;

use crate::desktop_system::{
    DesktopPaths, DesktopSystemError, DesktopSystemResult, PickFilesOptions,
};
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
    history::{
        RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto,
        RunHistoryPageDto, RunHistoryQueryDto,
    },
    prompt::{
        CompilePromptRequestDto, CompiledPromptDto, DeletePromptChunkRequestDto,
        DeletePromptChunkResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
        PromptChunkDto, PromptChunkPageDto, PromptLexiconCatalogDto, PromptLexiconListQueryDto,
        PromptLexiconPageDto, PromptLexiconSearchQueryDto, UpsertPromptChunkRequestDto,
    },
    resource::{
        GetResourceImageRequestDto, ImageResourceKindDto, ImportImageResourceRequestDto,
        ImportImageResourceResponseDto, ResourceImageDto,
    },
    settings::{
        ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
    },
    vibe::{
        EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
        ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
        ImportedVibeDocumentsDto,
    },
    workspace::{CloseWorkspaceResponseDto, OpenWorkspaceRequestDto, WorkspaceStatusDto},
};
use tauri::State;

use crate::desktop::{DesktopState, TauriDialog, TauriPathOpener};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDesktopFileDto {
    pub path: PathBuf,
}

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
pub async fn pick_and_import_image_resources(
    state: State<'_, DesktopState>,
    kind: ImageResourceKindDto,
    options: PickFilesOptions,
) -> CommandResult<Vec<ImportImageResourceResponseDto>> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    let files = desktop_result(state.system.pick_image_files(&dialog, options))?;
    let requests = desktop_result(
        files
            .iter()
            .map(|path| image_resource_request_from_file(path, kind))
            .collect::<DesktopSystemResult<Vec<_>>>(),
    )?;
    let mut imported = Vec::with_capacity(requests.len());

    for request in requests {
        imported.push(state.host.import_image_resource(request).await?);
    }

    Ok(imported)
}

#[tauri::command]
pub async fn pick_and_import_vibe_documents(
    state: State<'_, DesktopState>,
    options: PickFilesOptions,
) -> CommandResult<ImportedVibeDocumentsDto> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    let files = desktop_result(state.system.pick_vibe_documents(&dialog, options))?;
    let requests = desktop_result(
        files
            .iter()
            .map(|path| vibe_document_request_from_file(path))
            .collect::<DesktopSystemResult<Vec<_>>>(),
    )?;
    let mut entries = Vec::new();

    for request in requests {
        let imported = state.host.import_vibe_document(request).await?;
        entries.extend(imported.entries);
    }

    Ok(ImportedVibeDocumentsDto { entries })
}

#[tauri::command]
pub async fn pick_and_import_embedded_png_vibe_documents(
    state: State<'_, DesktopState>,
    options: PickFilesOptions,
) -> CommandResult<ImportedVibeDocumentsDto> {
    let dialog = TauriDialog::new(state.app_handle.clone());
    let files = desktop_result(state.system.pick_png_files(&dialog, options))?;
    let requests = desktop_result(
        files
            .iter()
            .map(|path| embedded_png_vibe_document_request_from_file(path))
            .collect::<DesktopSystemResult<Vec<_>>>(),
    )?;
    let mut entries = Vec::new();

    for request in requests {
        let imported = state
            .host
            .import_embedded_png_vibe_document(request)
            .await?;
        entries.extend(imported.entries);
    }

    Ok(ImportedVibeDocumentsDto { entries })
}

#[tauri::command]
pub async fn save_vibe_document(
    state: State<'_, DesktopState>,
    request: ExportVibeDocumentRequestDto,
) -> CommandResult<Option<SavedDesktopFileDto>> {
    let exported = state.host.export_vibe_document(request).await?;
    let dialog = TauriDialog::new(state.app_handle.clone());
    let default_file_name = format!("vibes.{}", exported.file_extension);
    let Some(path) =
        desktop_result(dialog.save_file(Some(&default_file_name), Some(&exported.file_extension)))?
    else {
        return Ok(None);
    };

    desktop_result(write_file_text(&path, &exported.content))?;
    desktop_result(state.system.allow_user_path(&path))?;
    Ok(Some(SavedDesktopFileDto { path }))
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
pub async fn get_resource_image(
    state: State<'_, DesktopState>,
    request: GetResourceImageRequestDto,
) -> CommandResult<ResourceImageDto> {
    state.host.get_resource_image(request).await
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
pub async fn query_run_history(
    state: State<'_, DesktopState>,
    request: RunHistoryQueryDto,
) -> CommandResult<RunHistoryPageDto> {
    state.host.query_run_history(request).await
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

fn image_resource_request_from_file(
    path: &Path,
    kind: ImageResourceKindDto,
) -> DesktopSystemResult<ImportImageResourceRequestDto> {
    Ok(ImportImageResourceRequestDto {
        kind,
        image_base64: STANDARD.encode(read_file_bytes(path)?),
        mime_type: None,
    })
}

fn vibe_document_request_from_file(
    path: &Path,
) -> DesktopSystemResult<ImportVibeDocumentRequestDto> {
    Ok(ImportVibeDocumentRequestDto {
        file_name: file_name(path),
        content: read_file_text(path)?,
    })
}

fn embedded_png_vibe_document_request_from_file(
    path: &Path,
) -> DesktopSystemResult<ImportEmbeddedPngVibeDocumentRequestDto> {
    Ok(ImportEmbeddedPngVibeDocumentRequestDto {
        file_name: file_name(path),
        png_bytes_base64: STANDARD.encode(read_file_bytes(path)?),
    })
}

fn read_file_bytes(path: &Path) -> DesktopSystemResult<Vec<u8>> {
    fs::read(path).map_err(|error| {
        DesktopSystemError::new(format!("failed to read file {}: {error}", path.display()))
    })
}

fn read_file_text(path: &Path) -> DesktopSystemResult<String> {
    fs::read_to_string(path).map_err(|error| {
        DesktopSystemError::new(format!(
            "failed to read text file {}: {error}",
            path.display()
        ))
    })
}

fn write_file_text(path: &Path, content: &str) -> DesktopSystemResult<()> {
    fs::write(path, content).map_err(|error| {
        DesktopSystemError::new(format!("failed to write file {}: {error}", path.display()))
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("document")
        .to_owned()
}

fn desktop_result<T>(result: DesktopSystemResult<T>) -> CommandResult<T> {
    result.map_err(|error| ErrorEnvelopeDto::new("desktop_system_error", error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_resource_request_reads_selected_file_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("source.png");
        fs::write(&path, [0x89, b'P', b'N', b'G']).unwrap();

        let request =
            image_resource_request_from_file(&path, ImageResourceKindDto::SourceImage).unwrap();

        assert_eq!(request.kind, ImageResourceKindDto::SourceImage);
        assert_eq!(request.image_base64, "iVBORw==");
        assert_eq!(request.mime_type, None);
    }

    #[test]
    fn vibe_document_request_reads_selected_json_text() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("example.naiv4vibe");
        fs::write(&path, r#"{"name":"example"}"#).unwrap();

        let request = vibe_document_request_from_file(&path).unwrap();

        assert_eq!(request.file_name, "example.naiv4vibe");
        assert_eq!(request.content, r#"{"name":"example"}"#);
    }

    #[test]
    fn embedded_png_vibe_document_request_reads_selected_png_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("embedded.png");
        fs::write(&path, [1, 2, 3, 4]).unwrap();

        let request = embedded_png_vibe_document_request_from_file(&path).unwrap();

        assert_eq!(request.file_name, "embedded.png");
        assert_eq!(request.png_bytes_base64, "AQIDBA==");
    }

    #[test]
    fn file_read_errors_map_to_stable_desktop_error_code() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing.png");

        let error = desktop_result::<Vec<u8>>(read_file_bytes(&path)).unwrap_err();

        assert_eq!(error.code, "desktop_system_error");
        assert!(error.message.contains("failed to read file"));
    }
}
