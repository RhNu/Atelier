mod app_update;
mod desktop_io;
mod resource_image;

pub use app_update::*;
pub use desktop_io::*;

use atelier_app_api::downloadable_resource::DownloadableResourceInstallProgressDto;
use atelier_app_api::error::ErrorEnvelopeDto;
use atelier_downloadable_resources::{ResourceInstallProgress, ResourceInstallProgressSink};
use tauri::ipc::Channel;

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
mod workspace;
pub use workspace::*;

mod explore;
pub use explore::*;

mod downloads;
pub use downloads::*;

mod account;
pub use account::*;

mod agent;
pub use agent::*;

mod prompt;
pub use prompt::*;

mod generation;
pub use generation::*;

mod resources;
pub use resources::*;

mod resource_library;
pub use resource_library::*;

mod history;
pub use history::*;

mod vibe;
pub use vibe::*;

mod gallery;
pub use gallery::*;
