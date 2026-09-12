use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::gallery::{
    DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryImageReferenceDto,
    GalleryImageReferenceRequestDto, GalleryItemDetailDto, GalleryItemDetailRequestDto,
    GalleryItemDto, GalleryPageDto, GalleryQueryDto, RescanGallerySafetyRequestDto,
    RescanGallerySafetyResponseDto, SetGallerySafetyOverrideRequestDto,
};
use tauri::State;

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
