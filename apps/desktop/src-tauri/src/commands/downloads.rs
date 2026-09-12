use super::ResourceProgressChannel;
use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::downloadable_resource::{
    DownloadableResourceGroupRequestDto, DownloadableResourceInstallProgressDto,
    DownloadableResourceRequestDto, DownloadableResourceStatusDto, DownloadableResourcesDto,
};
use atelier_app_api::gallery::RescanGallerySafetyRequestDto;
use tauri::State;
use tauri::ipc::Channel;

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
