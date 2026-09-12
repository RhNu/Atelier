use std::sync::{Arc, Mutex as StdMutex};

use atelier_adapter_database::{
    DatabaseGalleryIndex, DatabaseGenerationDraftRepository, DatabaseGenerationStore,
    DatabasePromptResourceRepository, DatabaseRunHistoryRepository, DatabaseSettingsRepository,
};
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_novelai::{NovelAiEmbeddedVibeExtractor, ReqwestNovelAiClientFactory};
use atelier_kernel::KernelRuntime;
use atelier_prompt_resources::{PromptChunkService, PromptCompiler, PromptPresetService};
use atelier_safety::SafetyScanner;
use atelier_settings::WorkspaceSettingsService;
use atelier_workspace::{WorkspaceLockLease, WorkspaceRoot};
use futures::lock::Mutex;

use crate::events::AppEventHub;
use crate::ports::{
    AppApiKeyService, AppArtifactService, AppGalleryService, AppKernelPorts, AppResourceCatalog,
    AppResourceReader, SharedWorkspaceSettings,
};
use crate::usecases::{
    DirectorUseCases, EventsUseCases, GalleryUseCases, GenerationUseCases, HistoryUseCases,
    PromptUseCases, ResourceUseCases, SettingsUseCases, VibeUseCases, WorkspaceUseCases,
};
use crate::{AppResult, error::AppError};

pub struct WorkspaceSession<
    S = KeyringSecretStore,
    F = ReqwestNovelAiClientFactory,
    E = NovelAiEmbeddedVibeExtractor,
> {
    pub(crate) root: WorkspaceRoot,
    pub(crate) schema_version: u32,
    pub(crate) workspace_lock: StdMutex<Box<dyn WorkspaceLockLease>>,
    pub(crate) api_keys: AppApiKeyService<S, F>,
    pub(crate) settings: WorkspaceSettingsService<DatabaseSettingsRepository>,
    pub(crate) generation_drafts:
        atelier_generation::GenerationDraftService<DatabaseGenerationDraftRepository>,
    pub(crate) generation_draft_write: Mutex<()>,
    pub(crate) gallery_safety_rescan: Mutex<()>,
    pub(crate) settings_state: SharedWorkspaceSettings,
    pub(crate) prompt_chunks: PromptChunkService<DatabasePromptResourceRepository>,
    pub(crate) prompt_presets: PromptPresetService<DatabasePromptResourceRepository>,
    pub(crate) prompt_compiler: PromptCompiler<DatabasePromptResourceRepository>,
    pub(crate) prompt_resource_write: Mutex<()>,
    pub(crate) artifacts: AppArtifactService,
    pub(crate) gallery: AppGalleryService,
    pub(crate) gallery_index: DatabaseGalleryIndex,
    pub(crate) resources: AppResourceCatalog,
    pub(crate) resource_reader: AppResourceReader,
    pub(crate) safety_scanner: Option<Arc<dyn SafetyScanner>>,
    pub(crate) queue_repository: DatabaseGenerationStore,
    pub(crate) run_history: DatabaseRunHistoryRepository,
    pub(crate) queue: atelier_kernel::QueueView,
    pub(crate) workflows: atelier_kernel::WorkflowContext<AppKernelPorts<S, F, E>>,
    pub(crate) kernel: Mutex<KernelRuntime<AppKernelPorts<S, F, E>>>,
    pub(crate) events: AppEventHub,
}

impl<S, F, E> WorkspaceSession<S, F, E> {
    /// Explicitly releases the workspace lease before the session is dropped.
    ///
    /// This is intentionally separate from `Drop`: desktop shutdown may end
    /// the Tauri event loop before all `Arc` clones have been destroyed.
    ///
    /// # Errors
    /// Returns an error when the underlying workspace lock cannot be released.
    pub fn release_workspace_lock(&self) -> AppResult<()> {
        let mut lease = self.workspace_lock.lock().map_err(|_| {
            AppError::new(
                "workspace_lock_state_poisoned",
                "workspace lock state is unavailable",
            )
        })?;
        lease.release().map_err(AppError::from)
    }

    #[must_use]
    pub const fn workspace(&self) -> WorkspaceUseCases<'_> {
        WorkspaceUseCases {
            root: &self.root,
            schema_version: &self.schema_version,
        }
    }

    #[must_use]
    pub const fn prompt(&self) -> PromptUseCases<'_> {
        PromptUseCases {
            prompt_chunks: &self.prompt_chunks,
            prompt_compiler: &self.prompt_compiler,
            prompt_presets: &self.prompt_presets,
            prompt_resource_write: &self.prompt_resource_write,
            resources: &self.resources,
        }
    }

    #[must_use]
    pub const fn resources(&self) -> ResourceUseCases<'_> {
        ResourceUseCases {
            resource_reader: &self.resource_reader,
            resources: &self.resources,
        }
    }

    #[must_use]
    pub const fn settings(&self) -> SettingsUseCases<'_> {
        SettingsUseCases {
            settings: &self.settings,
            settings_state: &self.settings_state,
        }
    }

    #[must_use]
    pub const fn generation(&self) -> GenerationUseCases<'_, S, F, E> {
        GenerationUseCases { app: self }
    }

    #[must_use]
    pub const fn director(&self) -> DirectorUseCases<'_, S, F, E> {
        DirectorUseCases { app: self }
    }

    #[must_use]
    pub const fn vibe(&self) -> VibeUseCases<'_, S, F, E> {
        VibeUseCases { app: self }
    }

    #[must_use]
    pub const fn gallery(&self) -> GalleryUseCases<'_> {
        GalleryUseCases {
            artifacts: &self.artifacts,
            gallery: &self.gallery,
            gallery_index: &self.gallery_index,
            gallery_safety_rescan: &self.gallery_safety_rescan,
            resource_reader: &self.resource_reader,
            resources: &self.resources,
            safety_scanner: &self.safety_scanner,
        }
    }

    #[must_use]
    pub const fn history(&self) -> HistoryUseCases<'_, S, F, E> {
        HistoryUseCases { app: self }
    }

    #[must_use]
    pub const fn events(&self) -> EventsUseCases<'_> {
        EventsUseCases {
            events: &self.events,
        }
    }
}
