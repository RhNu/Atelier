use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, GetVibeDocumentRequestDto,
    ListVibeDocumentsRequestDto, RenameVibeDocumentRequestDto, SetVibeDocumentHiddenRequestDto,
    VibeDocumentEntryDto, VibeDocumentPageDto,
};
use tauri::State;

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
