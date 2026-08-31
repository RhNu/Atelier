use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::desktop_system::{
    DesktopFileDialog, DesktopNotifier, DesktopPathOpener, DesktopPaths, DesktopSystem,
    DesktopSystemError, DesktopSystemResult, PickFilesOptions,
};
use atelier_adapter_danbooru::ReqwestDanbooruClient;
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_lexicon_bundle::ManagedLexiconBundle;
use atelier_adapter_novelai::{NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory};
use atelier_adapter_novelai_explore::NovelAiExploreClient;
use atelier_adapter_secrets_fs::FileSystemApiKeyRegistryStore;
use atelier_adapter_settings_fs::FileSystemGlobalSettingsRepository;
use atelier_app::{AtelierRuntime, GenerationWorkerCancel};
use atelier_app_api::event::{AppEventDto, AppEventKindDto};
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_app_api::settings::FrontendLanguageDto;
use atelier_prompt_lexicon::LexiconEngine;
use atelier_safety::SafetyPipeline;
use atelier_settings::{FrontendLanguage, GlobalSettingsRepository, GlobalSettingsService};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

pub type NativeAtelierRuntime =
    AtelierRuntime<KeyringSecretStore, ReqwestNovelAiClientFactory, NovelAiEmbeddedVibeExtractor>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
enum NotificationLanguage {
    #[default]
    English,
    SimplifiedChinese,
}

impl NotificationLanguage {
    const fn from_domain(language: FrontendLanguage) -> Self {
        match language {
            FrontendLanguage::SimplifiedChinese => Self::SimplifiedChinese,
            FrontendLanguage::System | FrontendLanguage::English => Self::English,
        }
    }

    const fn from_dto(language: FrontendLanguageDto) -> Self {
        match language {
            FrontendLanguageDto::SimplifiedChinese => Self::SimplifiedChinese,
            FrontendLanguageDto::System | FrontendLanguageDto::English => Self::English,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GenerationNotificationKind {
    Completed,
    Failed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct GenerationNotification {
    title: &'static str,
    body: &'static str,
}

const fn generation_notification(
    language: NotificationLanguage,
    kind: GenerationNotificationKind,
) -> GenerationNotification {
    match (language, kind) {
        (NotificationLanguage::English, GenerationNotificationKind::Completed) => {
            GenerationNotification {
                title: "Atelier generation complete",
                body: "Your generated image is ready.",
            }
        }
        (NotificationLanguage::English, GenerationNotificationKind::Failed) => {
            GenerationNotification {
                title: "Atelier generation failed",
                body: "Open Generation to view details.",
            }
        }
        (NotificationLanguage::SimplifiedChinese, GenerationNotificationKind::Completed) => {
            GenerationNotification {
                title: "Atelier 生成完成",
                body: "生成的图像已准备就绪。",
            }
        }
        (NotificationLanguage::SimplifiedChinese, GenerationNotificationKind::Failed) => {
            GenerationNotification {
                title: "Atelier 生成失败",
                body: "请打开“生成”查看详情。",
            }
        }
    }
}

fn current_notification_language(language: &Mutex<NotificationLanguage>) -> NotificationLanguage {
    language.lock().map_or_default(|current| *current)
}

pub struct DesktopState {
    pub app_handle: AppHandle,
    pub host: Arc<NativeAtelierRuntime>,
    pub system: Arc<DesktopSystem>,
    pub worker: DesktopGenerationWorker,
    notification_language: Arc<Mutex<NotificationLanguage>>,
}

impl DesktopState {
    pub fn kick_generation_worker(&self, directive: QueueDirectiveDto) {
        self.worker.kick(
            self.app_handle.clone(),
            self.host.clone(),
            self.system.clone(),
            self.notification_language.clone(),
            directive,
        );
    }

    pub(crate) fn set_notification_language(&self, language: FrontendLanguageDto) {
        if let Ok(mut current) = self.notification_language.lock() {
            *current = NotificationLanguage::from_dto(language);
        }
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

    pub async fn shutdown(&self) {
        self.abort_generation_worker_and_wait().await;
        if let Err(error) = self.host.close_workspace() {
            log::warn!(
                "failed to close workspace during shutdown: {}",
                error.message
            );
        }
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
    const fn is_busy(&self) -> bool {
        self.current.is_some() || self.pending.is_some()
    }

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
        if let Some(run) = &mut self.current
            && run.id == run_id
        {
            run.handle = Some(handle);
        }
    }
}

impl DesktopGenerationWorker {
    pub fn is_busy(&self) -> bool {
        self.inner.lock().map_or(true, |state| state.is_busy())
    }

    fn kick(
        &self,
        app_handle: AppHandle,
        host: Arc<NativeAtelierRuntime>,
        system: Arc<DesktopSystem>,
        notification_language: Arc<Mutex<NotificationLanguage>>,
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

        self.spawn_run(app_handle, host, system, notification_language, start);
    }

    fn spawn_run(
        &self,
        app_handle: AppHandle,
        host: Arc<NativeAtelierRuntime>,
        system: Arc<DesktopSystem>,
        notification_language: Arc<Mutex<NotificationLanguage>>,
        start: WorkerStart,
    ) {
        let run_id = start.id;
        let worker = self.clone();
        let handle = tauri::async_runtime::spawn(async move {
            let result = host
                .drive_generation_queue(start.directive, start.cancel)
                .await;
            if let Err(error) = result {
                let notification = generation_notification(
                    current_notification_language(&notification_language),
                    GenerationNotificationKind::Failed,
                );
                let notifier = TauriNotifier::new(app_handle.clone());
                let _ = system.notify(notification.title, notification.body, &notifier);
                log::warn!("generation worker stopped with error: {}", error.message);
            }
            if let Some(next_directive) = worker.finish(run_id) {
                worker.kick(
                    app_handle,
                    host,
                    system,
                    notification_language,
                    next_directive,
                );
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
    let system = Arc::new(DesktopSystem::new(resolve_desktop_paths(&app_handle)?));
    let settings_repository = Arc::new(FileSystemGlobalSettingsRepository::new(
        system.paths().app_config_dir.join("global-settings.json"),
    ));
    let initial_settings =
        tauri::async_runtime::block_on(settings_repository.get_global_settings())
            .unwrap_or_default();
    let notification_language = Arc::new(Mutex::new(NotificationLanguage::from_domain(
        initial_settings.frontend.language,
    )));
    let global_settings = GlobalSettingsService::new(settings_repository);

    let catalog_url = std::env::var("ATELIER_RESOURCE_CATALOG_URL")
        .ok()
        .or_else(|| option_env!("ATELIER_RESOURCE_CATALOG_URL").map(str::to_owned))
        .unwrap_or_default();
    let downloadable_resources =
        atelier_adapter_downloadable_resources_fs::FileSystemDownloadableResourceManager::new(
            system.paths().app_data_dir.join("downloadable-resources"),
            catalog_url,
            "",
        )?;
    downloadable_resources.cleanup_legacy_image_analysis(&system.paths().app_data_dir)?;

    let image_analysis = system
        .resolve_onnx_runtime_library()
        .ok()
        .flatten()
        .and_then(|runtime_path| {
            let runtime =
                atelier_adapter_image_analysis_onnx::initialize_ort_runtime(&runtime_path)
                    .map_err(|error| error.to_string());
            runtime
                .and_then(|runtime| {
                    atelier_adapter_image_analysis_onnx::OnnxImageAnalysisRuntime::new(
                        runtime,
                        &runtime_path,
                        downloadable_resources.clone(),
                    )
                    .map_err(|error| error.to_string())
                })
                .map_err(|error| {
                    log::warn!("image analysis is unavailable: {error}");
                    error
                })
                .ok()
        });
    let lexicon: Arc<dyn LexiconEngine> = ManagedLexiconBundle::new(downloadable_resources.clone());
    let safety_pipeline = image_analysis.as_ref().map(|analysis| {
        Arc::new(SafetyPipeline::new(
            analysis.clone(),
            initial_settings.safety.wd_auto_review_enabled,
        ))
    });
    let mut runtime =
        AtelierRuntime::with_global_settings_dependencies_extractor_safety_and_lexicon(
            global_settings,
            KeyringSecretStore::native()?,
            ReqwestNovelAiClientFactory::default(),
            NovelAiEmbeddedVibeExtractor,
            safety_pipeline
                .clone()
                .map(|pipeline| pipeline as Arc<dyn atelier_safety::SafetyScanner>),
            lexicon,
        )
        .with_downloadable_resources(downloadable_resources)
        .with_api_key_registry(Arc::new(application_api_key_registry(&system)))
        .with_danbooru_client(Arc::new(ReqwestDanbooruClient::new()?))
        .with_novelai_explore_source(Arc::new(NovelAiExploreClient::new()?));
    if let Some(analysis) = image_analysis {
        runtime = runtime.with_image_analysis(
            analysis,
            safety_pipeline.expect("image analysis and safety pipeline are initialized together"),
        );
    }
    let host = Arc::new(runtime);
    subscribe_window_events(
        &host,
        app_handle.clone(),
        system.clone(),
        notification_language.clone(),
    )?;
    Ok(DesktopState {
        app_handle,
        host,
        system,
        worker: DesktopGenerationWorker::default(),
        notification_language,
    })
}

fn application_api_key_registry(system: &DesktopSystem) -> FileSystemApiKeyRegistryStore {
    FileSystemApiKeyRegistryStore::new(system.paths().app_config_dir.join("novelai-api-keys.json"))
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
    host: &NativeAtelierRuntime,
    app_handle: AppHandle,
    system: Arc<DesktopSystem>,
    notification_language: Arc<Mutex<NotificationLanguage>>,
) -> Result<(), Box<dyn std::error::Error>> {
    host.subscribe_events(Arc::new(move |event| {
        if let Err(error) = app_handle.emit("atelier-event", event.clone()) {
            log::warn!("failed to emit app event to window: {error}");
        }
        notify_for_generation_event(&system, &app_handle, &notification_language, &event);
    }))
    .map_err(|error| std::io::Error::other(error.message))?;
    Ok(())
}

fn notify_for_generation_event(
    system: &DesktopSystem,
    app_handle: &AppHandle,
    notification_language: &Mutex<NotificationLanguage>,
    event: &AppEventDto,
) {
    let kind = match &event.kind {
        AppEventKindDto::JobSucceeded { .. } => GenerationNotificationKind::Completed,
        AppEventKindDto::JobFailed { .. } => GenerationNotificationKind::Failed,
        _ => return,
    };
    let notification =
        generation_notification(current_notification_language(notification_language), kind);
    let notifier = TauriNotifier::new(app_handle.clone());
    let _ = system.notify(notification.title, notification.body, &notifier);
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
        let Some(files) = builder.blocking_pick_files() else {
            return Ok(Vec::new());
        };
        files.into_iter().map(file_path_to_path).collect()
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

    use super::{
        GenerationNotificationKind, NotificationLanguage, WorkerState, generation_notification,
    };

    #[test]
    fn generation_notifications_are_localized_and_do_not_include_job_ids() {
        let english = generation_notification(
            NotificationLanguage::English,
            GenerationNotificationKind::Completed,
        );
        assert_eq!(english.title, "Atelier generation complete");
        assert_eq!(english.body, "Your generated image is ready.");
        assert!(!english.body.contains("job"));

        let chinese = generation_notification(
            NotificationLanguage::SimplifiedChinese,
            GenerationNotificationKind::Failed,
        );
        assert_eq!(chinese.title, "Atelier 生成失败");
        assert_eq!(chinese.body, "请打开“生成”查看详情。");
        assert!(!chinese.body.contains("job"));
    }

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

        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );
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
        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );

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
        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );

        assert!(state.take_current_for_abort().is_none());
        assert!(state.current.is_none());
        assert!(state.pending.is_none());
        assert!(first.cancel.is_cancelled());
    }

    #[test]
    fn worker_state_reports_active_and_queued_work_as_busy_for_updater_guard() {
        let mut state = WorkerState::default();
        assert!(!state.is_busy());
        let current = state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned(),
            })
            .unwrap();
        assert!(state.is_busy());
        state.cancel_current();
        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );
        assert!(state.is_busy());
        assert!(state.finish(current.id).is_some());
        assert!(!state.is_busy());
    }
}
