use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_novelai::{NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory};
use atelier_agent::{AgentModelRuntime, AgentRegistryRepository, UnavailableAgentModelRuntime};
use atelier_danbooru::{DanbooruClient, UnavailableDanbooruClient};
use atelier_downloadable_resources::DownloadableResourceManager;
use atelier_image_analysis::ImageAnalysisSessionControl;
use atelier_prompt_lexicon::{LexiconEngine, UnavailableLexicon};
use atelier_safety::{SafetyPolicyControl, SafetyScanner};
use atelier_secrets::ApiKeyRegistryStore;
use atelier_settings::GlobalSettingsService;
use std::sync::Arc;

/// Explicit process dependencies. Persistence is required even when no workspace is open.
pub struct RuntimeDependencies<
    S = KeyringSecretStore,
    F = ReqwestNovelAiClientFactory,
    E = NovelAiEmbeddedVibeExtractor,
> {
    pub secrets: S,
    pub factory: F,
    pub extractor: E,
    pub global_settings: GlobalSettingsService,
    pub api_key_registry: Arc<dyn ApiKeyRegistryStore>,
    pub agent_registry: Arc<dyn AgentRegistryRepository>,
    pub agent_runtime: Arc<dyn AgentModelRuntime>,
    pub safety_scanner: Option<Arc<dyn SafetyScanner>>,
    pub downloadable_resources: Option<Arc<dyn DownloadableResourceManager>>,
    pub image_analysis: Option<ImageAnalysisDependencies>,
    pub lexicon: Arc<dyn LexiconEngine>,
    pub danbooru: Arc<dyn DanbooruClient>,
    pub novelai_explore: Option<Arc<crate::explore::NovelAiExploreSource>>,
}

pub struct ImageAnalysisDependencies {
    pub sessions: Arc<dyn ImageAnalysisSessionControl>,
    pub policy: Arc<dyn SafetyPolicyControl>,
}

impl<S, F, E> RuntimeDependencies<S, F, E> {
    #[must_use]
    pub fn new(
        secrets: S,
        factory: F,
        extractor: E,
        global_settings: GlobalSettingsService,
        api_key_registry: Arc<dyn ApiKeyRegistryStore>,
        agent_registry: Arc<dyn AgentRegistryRepository>,
    ) -> Self {
        Self {
            secrets,
            factory,
            extractor,
            global_settings,
            api_key_registry,
            agent_registry,
            agent_runtime: Arc::new(UnavailableAgentModelRuntime),
            safety_scanner: None,
            downloadable_resources: None,
            image_analysis: None,
            lexicon: Arc::new(UnavailableLexicon::default()),
            danbooru: Arc::new(UnavailableDanbooruClient),
            novelai_explore: None,
        }
    }
}
