use atelier_app::CommandResult;
use atelier_app_api::error::ErrorEnvelopeDto;
use serde::Serialize;
use tauri::{State, ipc::Channel};
use tauri_plugin_updater::UpdaterExt;

use crate::desktop::DesktopState;

#[derive(Clone, Debug, Serialize)]
pub struct AppUpdateDto {
    pub current_version: String,
    pub version: String,
    pub release_notes: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AppUpdateProgressDto {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[tauri::command]
pub async fn check_app_update(
    state: State<'_, DesktopState>,
) -> CommandResult<Option<AppUpdateDto>> {
    let updater = state.app_handle.updater().map_err(updater_error)?;
    updater
        .check()
        .await
        .map_err(updater_error)
        .map(|update| update.map(update_to_dto))
}

#[tauri::command]
pub async fn install_app_update(
    state: State<'_, DesktopState>,
    on_progress: Channel<AppUpdateProgressDto>,
) -> CommandResult<()> {
    if state.worker.is_busy() {
        return Err(ErrorEnvelopeDto::new(
            "app_update_generation_busy",
            "finish or stop the generation queue before installing an application update",
        ));
    }
    let update = state
        .app_handle
        .updater()
        .map_err(updater_error)?
        .check()
        .await
        .map_err(updater_error)?
        .ok_or_else(|| {
            ErrorEnvelopeDto::new("app_update_not_available", "no update is available")
        })?;
    let mut downloaded = 0_u64;
    update
        .download_and_install(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk as u64);
                let _ = on_progress.send(AppUpdateProgressDto {
                    downloaded_bytes: downloaded,
                    total_bytes: total,
                });
            },
            || {},
        )
        .await
        .map_err(updater_error)?;
    state.shutdown().await;
    state.app_handle.restart();
}

fn update_to_dto(update: tauri_plugin_updater::Update) -> AppUpdateDto {
    AppUpdateDto {
        current_version: update.current_version,
        version: update.version,
        release_notes: update.body,
        published_at: update.date.map(|date| date.to_string()),
    }
}

fn updater_error(error: impl std::fmt::Display) -> ErrorEnvelopeDto {
    ErrorEnvelopeDto::new("app_update_failed", error.to_string())
}
