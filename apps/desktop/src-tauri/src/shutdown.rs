use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Manager};

use crate::desktop::DesktopState;

/// Coordinates the asynchronous shutdown handshake with Tauri's synchronous
/// `ExitRequested` callback. Tauri may deliver the event more than once, so
/// the transition is guarded and the process exits only after cleanup runs.
#[derive(Clone, Default)]
pub struct ShutdownCoordinator {
    started: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    pub fn request(&self, app_handle: &AppHandle) {
        if self.completed.load(Ordering::Acquire) || self.started.swap(true, Ordering::AcqRel) {
            return;
        }

        let app_handle = app_handle.clone();
        let completed = self.completed.clone();
        tauri::async_runtime::spawn(async move {
            app_handle.state::<DesktopState>().shutdown().await;
            completed.store(true, Ordering::Release);
            app_handle.exit(0);
        });
    }
}
