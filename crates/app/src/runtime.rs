use crate::ports::AppApiKeyService;
use crate::{AppError, AppEventListener, AppResult, WorkspaceSession};
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_novelai::{
    NovelAiClientFactory, NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory,
};
use atelier_app_api::error::ErrorEnvelopeDto;
use atelier_app_api::event::AppEventDto;
use atelier_danbooru::DanbooruClient;
use atelier_downloadable_resources::DownloadableResourceManager;
use atelier_image_analysis::ImageAnalysisSessionControl;
use atelier_prompt_lexicon::LexiconEngine;
use atelier_safety::{SafetyPolicyControl, SafetyScanner};
use atelier_secrets::{ApiKeyRegistryService, SecretStore};
use atelier_settings::GlobalSettingsService;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use std::sync::{Arc, Mutex, MutexGuard};

pub type CommandResult<T> = Result<T, ErrorEnvelopeDto>;
type Session<S, F, E> = Option<Arc<WorkspaceSession<S, F, E>>>;
type SessionGuard<'a, S, F, E> = MutexGuard<'a, Session<S, F, E>>;

pub struct AtelierRuntime<
    S = KeyringSecretStore,
    F = ReqwestNovelAiClientFactory,
    E = NovelAiEmbeddedVibeExtractor,
> {
    pub(crate) session: Mutex<Session<S, F, E>>,
    pub(crate) secrets: S,
    pub(crate) factory: F,
    pub(crate) extractor: E,
    pub(crate) safety_scanner: Option<Arc<dyn SafetyScanner>>,
    pub(crate) downloadable_resources: Option<Arc<dyn DownloadableResourceManager>>,
    pub(crate) image_analysis_sessions: Option<Arc<dyn ImageAnalysisSessionControl>>,
    pub(crate) safety_policy_control: Option<Arc<dyn SafetyPolicyControl>>,
    pub(crate) lexicon: Arc<dyn LexiconEngine>,
    pub(crate) danbooru: Arc<dyn DanbooruClient>,
    pub(crate) novelai_explore: Option<Arc<crate::explore::NovelAiExploreSource>>,
    pub(crate) explore_identity_revision: std::sync::atomic::AtomicU64,
    pub(crate) danbooru_account_gate: futures::lock::Mutex<()>,
    pub(crate) event_listeners: Mutex<Vec<AppEventListener>>,
    pub(crate) global_settings: GlobalSettingsService,
    pub(crate) api_keys: AppApiKeyService<S, F>,
}

impl<S, F, E> AtelierRuntime<S, F, E> {
    #[must_use]
    pub fn new(dependencies: crate::RuntimeDependencies<S, F, E>) -> Self
    where
        S: Clone,
        F: Clone,
    {
        let api_keys = ApiKeyRegistryService::new(
            dependencies.api_key_registry,
            dependencies.secrets.clone(),
            atelier_adapter_novelai::NovelAiSubscriptionProbeClient::new(
                dependencies.factory.clone(),
            ),
        );
        let analysis = dependencies.image_analysis;
        Self {
            session: Mutex::new(None),
            secrets: dependencies.secrets,
            factory: dependencies.factory,
            extractor: dependencies.extractor,
            safety_scanner: dependencies.safety_scanner,
            downloadable_resources: dependencies.downloadable_resources,
            image_analysis_sessions: analysis.as_ref().map(|value| value.sessions.clone()),
            safety_policy_control: analysis.map(|value| value.policy),
            lexicon: dependencies.lexicon,
            danbooru: dependencies.danbooru,
            novelai_explore: dependencies.novelai_explore,
            explore_identity_revision: std::sync::atomic::AtomicU64::new(0),
            danbooru_account_gate: futures::lock::Mutex::new(()),
            event_listeners: Mutex::new(Vec::new()),
            global_settings: dependencies.global_settings,
            api_keys,
        }
    }

    /// Borrows the application-wide account use cases, independently of a workspace.
    #[must_use]
    pub const fn account(&self) -> crate::usecases::AccountUseCases<'_, S, F> {
        crate::usecases::AccountUseCases {
            api_keys: &self.api_keys,
        }
    }

    /// Returns the current workspace session for host-neutral use cases.
    ///
    /// # Errors
    /// Returns an error when no workspace is open or session state is unavailable.
    pub fn current_session(&self) -> CommandResult<Arc<WorkspaceSession<S, F, E>>> {
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
            app.events.subscribe(listener);
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
            WorkspaceSession::open(
                root,
                crate::composition::WorkspaceServices {
                    api_keys: self.api_keys.clone(),
                    factory: self.factory.clone(),
                    extractor: self.extractor.clone(),
                    safety_scanner: self.safety_scanner.clone(),
                },
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
            session.events.subscribe(listener);
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

fn workspace_not_open() -> ErrorEnvelopeDto {
    AppError::new("workspace_not_open", "workspace is not open").envelope()
}
