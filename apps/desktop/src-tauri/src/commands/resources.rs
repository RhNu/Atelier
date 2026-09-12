use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::director::{DirectorToolResultDto, RunDirectorToolRequestDto};
use atelier_app_api::event::{AppEventPageDto, EventsSinceRequestDto};
use atelier_app_api::resource::{
    GetResourceImageRequestDto, ReleaseImportedImageResourcesRequestDto,
    ReleaseImportedImageResourcesResponseDto, ResourceImageDto,
};
use tauri::State;

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
pub async fn run_director_tool(
    state: State<'_, DesktopState>,
    request: RunDirectorToolRequestDto,
) -> CommandResult<DirectorToolResultDto> {
    state.host.run_director_tool(request).await
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
