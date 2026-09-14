use crate::desktop_system::{DesktopPaths, DesktopSystem};
use atelier_adapter_agent_config_fs::FileSystemAgentRegistryRepository;
use atelier_adapter_danbooru::ReqwestDanbooruClient;
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_lexicon_bundle::ManagedLexiconBundle;
use atelier_adapter_novelai::{NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory};
use atelier_adapter_novelai_explore::NovelAiExploreClient;
use atelier_adapter_secrets_fs::FileSystemApiKeyRegistryStore;
use atelier_adapter_settings_fs::FileSystemGlobalSettingsRepository;
use atelier_app::AtelierRuntime;
use atelier_app_api::event::{AppEventDto, AppEventKindDto};
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_app_api::settings::FrontendLanguageDto;
use atelier_prompt_lexicon::LexiconEngine;
use atelier_safety::SafetyPipeline;
use atelier_settings::{FrontendLanguage, GlobalSettingsRepository, GlobalSettingsService};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

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

#[derive(Clone)]
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
    let mut dependencies = atelier_app::RuntimeDependencies::new(
        KeyringSecretStore::native(),
        ReqwestNovelAiClientFactory::default(),
        NovelAiEmbeddedVibeExtractor,
        global_settings,
        Arc::new(application_api_key_registry(&system)),
        Arc::new(FileSystemAgentRegistryRepository::new(
            system.paths().app_config_dir.join("agent-registry.json"),
        )),
    );
    dependencies.agent_runtime = Arc::new(atelier_adapter_agent_rig::RigAgentRuntime);
    dependencies.safety_scanner = safety_pipeline
        .clone()
        .map(|value| value as Arc<dyn atelier_safety::SafetyScanner>);
    dependencies.lexicon = lexicon;
    dependencies.downloadable_resources = Some(downloadable_resources);
    dependencies.danbooru = Arc::new(ReqwestDanbooruClient::new()?);
    dependencies.novelai_explore = Some(Arc::new(NovelAiExploreClient::new()?));
    dependencies.image_analysis =
        image_analysis.map(|sessions| atelier_app::ImageAnalysisDependencies {
            sessions,
            policy: safety_pipeline
                .expect("image analysis and safety pipeline are initialized together"),
        });
    let runtime = AtelierRuntime::new(dependencies);
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

mod platform;
mod worker;
pub use platform::{TauriDialog, TauriNotifier, TauriPathOpener};
pub use worker::DesktopGenerationWorker;
