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

use nai_atelier_adapter_keyring::KeyringSecretStore;
use nai_atelier_adapter_novelai::{
    NovelAiClientFactory, NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory,
};
use nai_atelier_app_api::{error::ErrorEnvelopeDto, event::AppEventDto};
use nai_atelier_safety::SafetyScanner;
use nai_atelier_secrets::SecretStore;
use nai_atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::{AppError, AppEventListener, AppResult, AtelierApp};

pub type CommandResult<T> = Result<T, ErrorEnvelopeDto>;
type Session<S, F, E> = Option<Arc<AtelierApp<S, F, E>>>;
type SessionGuard<'a, S, F, E> = MutexGuard<'a, Session<S, F, E>>;

pub struct AppCommandHost<
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
}

impl AppCommandHost<KeyringSecretStore, ReqwestNovelAiClientFactory, NovelAiEmbeddedVibeExtractor> {
    /// Creates a command host backed by the native keyring and `NovelAI` client.
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

impl<S, F> AppCommandHost<S, F, NovelAiEmbeddedVibeExtractor> {
    #[must_use]
    pub const fn with_dependencies(secrets: S, factory: F) -> Self {
        Self::with_dependencies_and_extractor(secrets, factory, NovelAiEmbeddedVibeExtractor)
    }
}

impl<S, F, E> AppCommandHost<S, F, E> {
    #[must_use]
    pub const fn with_dependencies_and_extractor(secrets: S, factory: F, extractor: E) -> Self {
        Self {
            session: Mutex::new(None),
            secrets,
            factory,
            extractor,
            safety_scanner: None,
            event_listeners: Mutex::new(Vec::new()),
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
        }
    }

    pub(crate) fn current_app(&self) -> CommandResult<Arc<AtelierApp<S, F, E>>> {
        self.lock_session()?
            .as_ref()
            .cloned()
            .ok_or_else(workspace_not_open)
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

impl<S, F, E> AppCommandHost<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(crate) async fn open_app(
        &self,
        request: nai_atelier_app_api::workspace::OpenWorkspaceRequestDto,
    ) -> CommandResult<Arc<AtelierApp<S, F, E>>> {
        let root = request.root;
        let same_root = self
            .lock_session()?
            .as_ref()
            .is_some_and(|app| app.inner.root.as_path() == root.as_path());
        if same_root {
            self.lock_session()?.take();
        }
        let app = Arc::new(
            AtelierApp::open_workspace_with_dependencies_and_extractor_and_safety_scanner(
                root,
                self.secrets.clone(),
                self.factory.clone(),
                self.extractor.clone(),
                self.safety_scanner.clone(),
            )
            .await
            .map_err(|error| error.envelope())?,
        );
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
            app.inner.events.subscribe(listener);
        }
        *self.lock_session()? = Some(app.clone());
        Ok(app)
    }
}

fn workspace_not_open() -> ErrorEnvelopeDto {
    AppError::new("workspace_not_open", "workspace is not open").envelope()
}
