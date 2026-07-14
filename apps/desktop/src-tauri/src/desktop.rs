use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::desktop_system::{
    DesktopFileDialog, DesktopNotifier, DesktopPathOpener, DesktopPaths, DesktopSystem,
    DesktopSystemError, DesktopSystemResult, PickFilesOptions,
};
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_novelai::{NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory};
use atelier_app::{AppCommandHost, GenerationWorkerCancel};
use atelier_app_api::event::{AppEventDto, AppEventKindDto};
use atelier_app_api::generation::QueueDirectiveDto;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

pub type NativeAppCommandHost =
    AppCommandHost<KeyringSecretStore, ReqwestNovelAiClientFactory, NovelAiEmbeddedVibeExtractor>;

pub struct DesktopState {
    pub app_handle: AppHandle,
    pub host: Arc<NativeAppCommandHost>,
    pub system: Arc<DesktopSystem>,
    pub worker: DesktopGenerationWorker,
}

impl DesktopState {
    pub fn kick_generation_worker(&self, directive: QueueDirectiveDto) {
        self.worker.kick(
            self.app_handle.clone(),
            self.host.clone(),
            self.system.clone(),
            directive,
        );
    }

    pub fn cancel_generation_worker(&self) {
        self.worker.cancel();
    }

    pub fn cancel_generation_worker_and_clear_pending(&self) {
        self.worker.cancel_and_clear_pending();
    }

    pub async fn abort_generation_worker_and_wait(&self) {
        self.worker.abort_and_wait().await;
    }
}

#[derive(Clone, Default)]
pub struct DesktopGenerationWorker {
    inner: Arc<Mutex<WorkerState>>,
}

#[derive(Default)]
struct WorkerState {
    current: Option<WorkerRun>,
    pending: Option<QueueDirectiveDto>,
    next_id: u64,
}

struct WorkerRun {
    id: u64,
    cancel: GenerationWorkerCancel,
    handle: Option<tauri::async_runtime::JoinHandle<()>>,
}

struct WorkerStart {
    id: u64,
    directive: QueueDirectiveDto,
    cancel: GenerationWorkerCancel,
}

impl WorkerState {
    fn start_or_defer(&mut self, directive: QueueDirectiveDto) -> Option<WorkerStart> {
        if let Some(run) = &self.current {
            if run.cancel.is_cancelled() {
                self.pending = Some(directive);
            }
            return None;
        }

        let run_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let cancel = GenerationWorkerCancel::new();
        self.current = Some(WorkerRun {
            id: run_id,
            cancel: cancel.clone(),
            handle: None,
        });
        Some(WorkerStart {
            id: run_id,
            directive,
            cancel,
        })
    }

    fn cancel_current(&self) {
        if let Some(run) = &self.current {
            run.cancel.cancel();
        }
    }

    fn cancel_current_and_clear_pending(&mut self) {
        self.cancel_current();
        self.pending = None;
    }

    fn take_current_for_abort(&mut self) -> Option<tauri::async_runtime::JoinHandle<()>> {
        self.pending = None;
        let mut run = self.current.take()?;
        run.cancel.cancel();
        run.handle.take()
    }

    fn finish(&mut self, run_id: u64) -> Option<QueueDirectiveDto> {
        if self.current.as_ref().is_some_and(|run| run.id == run_id) {
            self.current = None;
            return self.pending.take();
        }
        None
    }

    fn attach_handle(&mut self, run_id: u64, handle: tauri::async_runtime::JoinHandle<()>) {
        if let Some(run) = &mut self.current {
            if run.id == run_id {
                run.handle = Some(handle);
            }
        }
    }
}

impl DesktopGenerationWorker {
    fn kick(
        &self,
        app_handle: AppHandle,
        host: Arc<NativeAppCommandHost>,
        system: Arc<DesktopSystem>,
        directive: QueueDirectiveDto,
    ) {
        if !matches!(
            directive,
            QueueDirectiveDto::StartJob { .. } | QueueDirectiveDto::Wait { .. }
        ) {
            return;
        }

        let Some(start) = ({
            let Ok(mut state) = self.inner.lock() else {
                log::warn!("generation worker state is unavailable");
                return;
            };
            state.start_or_defer(directive)
        }) else {
            return;
        };

        self.spawn_run(app_handle, host, system, start);
    }

    fn spawn_run(
        &self,
        app_handle: AppHandle,
        host: Arc<NativeAppCommandHost>,
        system: Arc<DesktopSystem>,
        start: WorkerStart,
    ) {
        let run_id = start.id;
        let worker = self.clone();
        let handle = tauri::async_runtime::spawn(async move {
            let result = host
                .drive_generation_queue(start.directive, start.cancel)
                .await;
            if let Err(error) = result {
                let notifier = TauriNotifier::new(app_handle.clone());
                let _ = system.notify("Atelier generation failed", &error.message, &notifier);
                log::warn!("generation worker stopped with error: {}", error.message);
            }
            if let Some(next_directive) = worker.finish(run_id) {
                worker.kick(app_handle, host, system, next_directive);
            }
        });
        self.attach_handle(run_id, handle);
    }

    fn cancel(&self) {
        let Ok(state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return;
        };
        state.cancel_current();
    }

    fn cancel_and_clear_pending(&self) {
        let Ok(mut state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return;
        };
        state.cancel_current_and_clear_pending();
    }

    async fn abort_and_wait(&self) {
        let handle = {
            let Ok(mut state) = self.inner.lock() else {
                log::warn!("generation worker state is unavailable");
                return;
            };
            state.take_current_for_abort()
        };
        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }
    }

    fn finish(&self, run_id: u64) -> Option<QueueDirectiveDto> {
        let Ok(mut state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return None;
        };
        state.finish(run_id)
    }

    fn attach_handle(&self, run_id: u64, handle: tauri::async_runtime::JoinHandle<()>) {
        let Ok(mut state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return;
        };
        state.attach_handle(run_id, handle);
    }
}

pub fn build_desktop_state(
    app_handle: AppHandle,
) -> Result<DesktopState, Box<dyn std::error::Error>> {
    tauri::async_runtime::spawn_blocking(|| {
        if let Err(error) = atelier_app::preload_static_resources() {
            log::warn!("static workspace resources could not be preloaded: {error}");
        }
    });
    let system = Arc::new(DesktopSystem::new(resolve_desktop_paths(&app_handle)?));
    let safety_scanner = match system.resolve_safety_assets() {
        Ok(assets) => match atelier_adapter_safety_onnx::build_safety_scanner(assets) {
            Ok(scanner) => scanner,
            Err(error) => {
                log::warn!("safety scanner is unavailable: {error}");
                None
            }
        },
        Err(error) => {
            log::warn!("safety assets are unavailable: {error}");
            None
        }
    };
    let host = Arc::new(
        AppCommandHost::with_dependencies_extractor_and_safety_scanner(
            KeyringSecretStore::native()?,
            ReqwestNovelAiClientFactory::default(),
            NovelAiEmbeddedVibeExtractor,
            safety_scanner,
        ),
    );

    subscribe_window_events(&host, app_handle.clone(), system.clone())?;

    Ok(DesktopState {
        app_handle,
        host,
        system,
        worker: DesktopGenerationWorker::default(),
    })
}

fn resolve_desktop_paths(app_handle: &AppHandle) -> Result<DesktopPaths, tauri::Error> {
    let app_data_dir = app_handle.path().app_data_dir()?;
    let app_config_dir = app_handle.path().app_config_dir()?;
    let app_cache_dir = app_handle.path().app_cache_dir()?;
    let suggested_workspace_dir = app_handle.path().document_dir().map_or_else(
        |_| app_data_dir.join("workspaces"),
        |document_dir| document_dir.join("Atelier"),
    );
    let resource_dir = app_handle.path().resource_dir().ok();

    Ok(DesktopPaths {
        app_data_dir,
        app_config_dir,
        app_cache_dir,
        suggested_workspace_dir,
        resource_dir,
    })
}

fn subscribe_window_events(
    host: &NativeAppCommandHost,
    app_handle: AppHandle,
    system: Arc<DesktopSystem>,
) -> Result<(), Box<dyn std::error::Error>> {
    host.subscribe_events(Arc::new(move |event| {
        if let Err(error) = app_handle.emit("atelier-event", event.clone()) {
            log::warn!("failed to emit app event to window: {error}");
        }
        notify_for_generation_event(&system, &app_handle, &event);
    }))
    .map_err(|error| std::io::Error::other(error.message))?;
    Ok(())
}

fn notify_for_generation_event(
    system: &DesktopSystem,
    app_handle: &AppHandle,
    event: &AppEventDto,
) {
    let notifier = TauriNotifier::new(app_handle.clone());
    match &event.kind {
        AppEventKindDto::JobSucceeded { job_id, .. } => {
            let _ = system.notify(
                "Atelier generation complete",
                &format!("Job {job_id} finished."),
                &notifier,
            );
        }
        AppEventKindDto::JobFailed {
            job_id, message, ..
        } => {
            let _ = system.notify(
                "Atelier generation failed",
                &format!("Job {job_id} failed: {message}"),
                &notifier,
            );
        }
        _ => {}
    }
}

pub struct TauriDialog {
    app_handle: AppHandle,
}

impl TauriDialog {
    #[must_use]
    pub const fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn save_file(
        &self,
        default_file_name: Option<&str>,
        extension: Option<&str>,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        let mut builder = self.app_handle.dialog().file();
        if let Some(default_file_name) = default_file_name {
            builder = builder.set_file_name(default_file_name);
        }
        if let Some(extension) = extension {
            builder = builder.add_filter("Files", &[extension]);
        }

        builder
            .blocking_save_file()
            .map(file_path_to_path)
            .transpose()
    }
}

impl DesktopFileDialog for TauriDialog {
    fn pick_directory(&self) -> DesktopSystemResult<Option<PathBuf>> {
        self.app_handle
            .dialog()
            .file()
            .blocking_pick_folder()
            .map(file_path_to_path)
            .transpose()
    }

    fn pick_files(&self, options: PickFilesOptions) -> DesktopSystemResult<Vec<PathBuf>> {
        let mut builder = self.app_handle.dialog().file();
        if !options.extensions.is_empty() {
            let extension_refs: Vec<&str> = options.extensions.iter().map(String::as_str).collect();
            builder = builder.add_filter("Files", &extension_refs);
        }
        builder
            .blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .map(file_path_to_path)
            .collect()
    }
}

fn file_path_to_path(path: FilePath) -> DesktopSystemResult<PathBuf> {
    path.into_path().map_err(|error| {
        DesktopSystemError::new(format!(
            "selected path is not a local filesystem path: {error}"
        ))
    })
}

pub struct TauriPathOpener {
    app_handle: AppHandle,
}

impl TauriPathOpener {
    #[must_use]
    pub const fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl DesktopPathOpener for TauriPathOpener {
    fn open_path(&self, path: &Path) -> DesktopSystemResult<()> {
        self.app_handle
            .opener()
            .open_path(path.to_string_lossy().into_owned(), None::<String>)
            .map_err(|error| DesktopSystemError::new(format!("failed to open path: {error}")))
    }

    fn reveal_path(&self, path: &Path) -> DesktopSystemResult<()> {
        self.app_handle
            .opener()
            .reveal_item_in_dir(path)
            .map_err(|error| DesktopSystemError::new(format!("failed to reveal path: {error}")))
    }
}

pub struct TauriNotifier {
    app_handle: AppHandle,
}

impl TauriNotifier {
    #[must_use]
    pub const fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl DesktopNotifier for TauriNotifier {
    fn notify(&self, title: &str, body: &str) -> DesktopSystemResult<()> {
        self.app_handle
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| {
                DesktopSystemError::new(format!("failed to send notification: {error}"))
            })
    }
}

#[cfg(test)]
mod tests {
    use atelier_app_api::generation::{QueueDelayDto, QueueDirectiveDto};

    use super::WorkerState;

    #[test]
    fn worker_state_defers_cancelled_run_replacement_until_finish() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::Wait {
                delay: QueueDelayDto {
                    min_ms: 10,
                    max_ms: 10,
                },
            })
            .unwrap();

        state.cancel_current();
        let deferred = QueueDirectiveDto::StartJob {
            job_id: "job-2".to_owned(),
        };

        assert!(state.start_or_defer(deferred.clone()).is_none());
        assert_eq!(state.finish(first.id), Some(deferred.clone()));

        let second = state.start_or_defer(deferred).unwrap();
        assert_ne!(second.id, first.id);
    }

    #[test]
    fn worker_state_drops_kicks_while_active_run_is_not_cancelled() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned(),
            })
            .unwrap();

        assert!(state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-2".to_owned(),
            })
            .is_none());
        assert_eq!(state.finish(first.id), None);
    }

    #[test]
    fn worker_state_ignores_stale_finish_after_newer_run_started() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned(),
            })
            .unwrap();
        state.cancel_current();
        let deferred = QueueDirectiveDto::StartJob {
            job_id: "job-2".to_owned(),
        };
        assert!(state.start_or_defer(deferred.clone()).is_none());
        assert_eq!(state.finish(first.id), Some(deferred.clone()));
        let second = state.start_or_defer(deferred).unwrap();

        assert_eq!(state.finish(first.id), None);
        assert!(state.finish(second.id).is_none());
    }

    #[test]
    fn worker_state_clears_pending_for_terminal_cancel() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::Wait {
                delay: QueueDelayDto {
                    min_ms: 10,
                    max_ms: 10,
                },
            })
            .unwrap();
        state.cancel_current();
        assert!(state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-2".to_owned(),
            })
            .is_none());

        state.cancel_current_and_clear_pending();

        assert_eq!(state.finish(first.id), None);
    }

    #[test]
    fn worker_state_abort_releases_current_run_and_clears_pending() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::Wait {
                delay: QueueDelayDto {
                    min_ms: 10,
                    max_ms: 10,
                },
            })
            .unwrap();
        state.cancel_current();
        assert!(state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-2".to_owned(),
            })
            .is_none());

        assert!(state.take_current_for_abort().is_none());
        assert!(state.current.is_none());
        assert!(state.pending.is_none());
        assert!(first.cancel.is_cancelled());
    }
}
