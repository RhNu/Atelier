use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager};

use crate::desktop::DesktopState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitRequestAction {
    BeginShutdown,
    WaitForShutdown,
    AllowExit,
}

/// Coordinates the asynchronous shutdown handshake with Tauri's synchronous
/// `ExitRequested` callback. Tauri may deliver the event more than once, so
/// the transition is guarded and the process exits only after cleanup runs.
#[derive(Clone, Default)]
pub struct ShutdownCoordinator {
    started: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    pub fn on_exit_requested(&self) -> ExitRequestAction {
        if self.completed.load(Ordering::Acquire) {
            ExitRequestAction::AllowExit
        } else if self.started.swap(true, Ordering::AcqRel) {
            ExitRequestAction::WaitForShutdown
        } else {
            ExitRequestAction::BeginShutdown
        }
    }

    pub fn begin(&self, app_handle: &AppHandle) {
        let app_handle = app_handle.clone();
        let completed = self.completed.clone();
        tauri::async_runtime::spawn(async move {
            app_handle.state::<DesktopState>().shutdown().await;
            completed.store(true, Ordering::Release);
            app_handle.exit(0);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use super::{ExitRequestAction, ShutdownCoordinator};

    #[test]
    fn first_exit_request_begins_shutdown_and_repeated_requests_wait() {
        let shutdown = ShutdownCoordinator::default();

        assert_eq!(
            shutdown.on_exit_requested(),
            ExitRequestAction::BeginShutdown
        );
        assert_eq!(
            shutdown.on_exit_requested(),
            ExitRequestAction::WaitForShutdown
        );
    }

    #[test]
    fn completed_shutdown_allows_the_follow_up_exit_request() {
        let shutdown = ShutdownCoordinator::default();
        assert_eq!(
            shutdown.on_exit_requested(),
            ExitRequestAction::BeginShutdown
        );

        shutdown.completed.store(true, Ordering::Release);

        assert_eq!(shutdown.on_exit_requested(), ExitRequestAction::AllowExit);
    }
}
