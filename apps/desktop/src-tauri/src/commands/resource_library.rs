use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::resource_library::{
    DeleteLibraryFolderRequestDto, DeleteLibraryFolderResponseDto, GetLibrarySnapshotRequestDto,
    LibraryFolderDto, LibraryResourceDto, LibrarySnapshotDto, UpdateLibraryResourceRequestDto,
    UpsertLibraryFolderRequestDto,
};
use tauri::State;

#[tauri::command]
pub async fn get_resource_library_snapshot(
    state: State<'_, DesktopState>,
    request: GetLibrarySnapshotRequestDto,
) -> CommandResult<LibrarySnapshotDto> {
    state.host.get_resource_library_snapshot(request).await
}

#[tauri::command]
pub async fn upsert_resource_library_folder(
    state: State<'_, DesktopState>,
    request: UpsertLibraryFolderRequestDto,
) -> CommandResult<LibraryFolderDto> {
    state.host.upsert_resource_library_folder(request).await
}

#[tauri::command]
pub async fn update_resource_library_resource(
    state: State<'_, DesktopState>,
    request: UpdateLibraryResourceRequestDto,
) -> CommandResult<LibraryResourceDto> {
    state.host.update_resource_library_resource(request).await
}

#[tauri::command]
pub async fn delete_resource_library_folder(
    state: State<'_, DesktopState>,
    request: DeleteLibraryFolderRequestDto,
) -> CommandResult<DeleteLibraryFolderResponseDto> {
    state.host.delete_resource_library_folder(request).await
}
