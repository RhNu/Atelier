use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::danbooru::{DanbooruAccountDto, SaveDanbooruAccountRequestDto};
use atelier_app_api::explore::{
    ExploreItemRefDto, ExploreMediaRequestDto, ExplorePageDto, ExplorePostDetailDto,
    ExploreSearchRequestDto, ExploreSourceDescriptorDto,
};
use atelier_app_api::resource::ResourceImageDto;
use tauri::State;

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
