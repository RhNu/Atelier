mod account;
mod director;
mod events;
mod gallery;
mod generation;
mod history;
mod prompt;
mod resource;
mod settings;
mod vibe;
mod workspace;

use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_novelai::{
    NovelAiClientFactory, NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory,
};
use atelier_app_api::{error::ErrorEnvelopeDto, event::AppEventDto};
use atelier_safety::SafetyScanner;
use atelier_secrets::SecretStore;
use atelier_settings::{
    GlobalSettings, GlobalSettingsRepository, GlobalSettingsService, SettingsResult,
};
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::{AppError, AppEventListener, AppResult, WorkspaceSession};

pub type CommandResult<T> = Result<T, ErrorEnvelopeDto>;
type Session<S, F, E> = Option<Arc<WorkspaceSession<S, F, E>>>;
type SessionGuard<'a, S, F, E> = MutexGuard<'a, Session<S, F, E>>;

pub struct AtelierRuntime<
    S = KeyringSecretStore,
    F = ReqwestNovelAiClientFactory,
    E = NovelAiEmbeddedVibeExtractor,
> {
    session: Mutex<Session<S, F, E>>,
    secrets: S,
    factory: F,
    extractor: E,
    safety_scanner: Option<Arc<dyn SafetyScanner>>,
    event_listeners: Mutex<Vec<AppEventListener>>,
    global_settings: GlobalSettingsService,
}

impl AtelierRuntime<KeyringSecretStore, ReqwestNovelAiClientFactory, NovelAiEmbeddedVibeExtractor> {
    /// Creates a runtime backed by the native keyring and `NovelAI` client.
    ///
    /// # Errors
    /// Returns an error when the native keyring adapter cannot be created.
    pub fn native() -> AppResult<Self> {
        Ok(Self::with_dependencies(
            KeyringSecretStore::native()?,
            ReqwestNovelAiClientFactory::default(),
        ))
    }
}

impl<S, F> AtelierRuntime<S, F, NovelAiEmbeddedVibeExtractor> {
    #[must_use]
    pub fn with_dependencies(secrets: S, factory: F) -> Self {
        Self::with_dependencies_and_extractor(secrets, factory, NovelAiEmbeddedVibeExtractor)
    }
}

impl<S, F, E> AtelierRuntime<S, F, E> {
    #[must_use]
    pub fn with_dependencies_and_extractor(secrets: S, factory: F, extractor: E) -> Self {
        Self {
            session: Mutex::new(None),
            secrets,
            factory,
            extractor,
            safety_scanner: None,
            event_listeners: Mutex::new(Vec::new()),
            global_settings: transient_global_settings_service(),
        }
    }

    #[must_use]
    pub fn with_dependencies_extractor_and_safety_scanner(
        secrets: S,
        factory: F,
        extractor: E,
        safety_scanner: Option<Arc<dyn SafetyScanner>>,
    ) -> Self {
        Self {
            session: Mutex::new(None),
            secrets,
            factory,
            extractor,
            safety_scanner,
            event_listeners: Mutex::new(Vec::new()),
            global_settings: transient_global_settings_service(),
        }
    }

    #[must_use]
    pub fn with_global_settings_dependencies_extractor_and_safety_scanner(
        global_settings: GlobalSettingsService,
        secrets: S,
        factory: F,
        extractor: E,
        safety_scanner: Option<Arc<dyn SafetyScanner>>,
    ) -> Self {
        Self {
            session: Mutex::new(None),
            secrets,
            factory,
            extractor,
            safety_scanner,
            event_listeners: Mutex::new(Vec::new()),
            global_settings,
        }
    }

    pub(crate) fn current_session(&self) -> CommandResult<Arc<WorkspaceSession<S, F, E>>> {
        self.lock_session()?
            .as_ref()
            .cloned()
            .ok_or_else(workspace_not_open)
    }

    pub(crate) fn current_session_optional(
        &self,
    ) -> CommandResult<Option<Arc<WorkspaceSession<S, F, E>>>> {
        Ok(self.lock_session()?.as_ref().cloned())
    }

    pub(crate) fn lock_session(&self) -> CommandResult<SessionGuard<'_, S, F, E>> {
        self.session.lock().map_err(|_| {
            ErrorEnvelopeDto::new(
                "command_state_poisoned",
                "app command session state is unavailable",
            )
        })
    }

    /// Subscribes to app events from the current and future workspace sessions.
    ///
    /// # Errors
    /// Returns an error envelope when event listener or session state is unavailable.
    pub fn subscribe_events(
        &self,
        listener: Arc<dyn Fn(AppEventDto) + Send + Sync + 'static>,
    ) -> CommandResult<()> {
        self.event_listeners
            .lock()
            .map_err(|_| {
                ErrorEnvelopeDto::new(
                    "command_state_poisoned",
                    "app event listener state is unavailable",
                )
            })?
            .push(listener.clone());
        if let Some(app) = self.lock_session()?.as_ref() {
            app.inner.events.subscribe(listener);
        }
        Ok(())
    }

    pub(crate) fn command_result<T>(result: AppResult<T>) -> CommandResult<T> {
        result.map_err(|error| error.envelope())
    }
}

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(crate) async fn build_session(
        &self,
        root: std::path::PathBuf,
    ) -> CommandResult<Arc<WorkspaceSession<S, F, E>>> {
        let session = Arc::new(
            WorkspaceSession::open_workspace_with_dependencies_and_extractor_and_safety_scanner(
                root,
                self.secrets.clone(),
                self.factory.clone(),
                self.extractor.clone(),
                self.safety_scanner.clone(),
            )
            .await
            .map_err(|error| error.envelope())?,
        );
        session
            .resources()
            .release_all_imported_images()
            .await
            .map_err(|error| error.envelope())?;
        let listeners = self
            .event_listeners
            .lock()
            .map_err(|_| {
                ErrorEnvelopeDto::new(
                    "command_state_poisoned",
                    "app event listener state is unavailable",
                )
            })?
            .clone();
        for listener in listeners {
            session.inner.events.subscribe(listener);
        }
        Ok(session)
    }

    pub(crate) fn publish_session(
        &self,
        session: Arc<WorkspaceSession<S, F, E>>,
    ) -> CommandResult<()> {
        *self.lock_session()? = Some(session);
        Ok(())
    }
}

#[derive(Default)]
struct TransientGlobalSettingsRepository {
    settings: Mutex<GlobalSettings>,
}

#[async_trait]
impl GlobalSettingsRepository for TransientGlobalSettingsRepository {
    async fn get_global_settings(&self) -> SettingsResult<GlobalSettings> {
        self.settings
            .lock()
            .map(|settings| settings.clone())
            .map_err(|_| {
                atelier_settings::SettingsError::repository("global settings state is unavailable")
            })
    }

    async fn save_global_settings(&self, settings: GlobalSettings) -> SettingsResult<()> {
        *self.settings.lock().map_err(|_| {
            atelier_settings::SettingsError::repository("global settings state is unavailable")
        })? = settings;
        Ok(())
    }
}

fn transient_global_settings_service() -> GlobalSettingsService {
    GlobalSettingsService::new(Arc::new(TransientGlobalSettingsRepository::default()))
}

fn workspace_not_open() -> ErrorEnvelopeDto {
    AppError::new("workspace_not_open", "workspace is not open").envelope()
}
