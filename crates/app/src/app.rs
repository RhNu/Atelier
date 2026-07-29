use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use atelier_adapter_database::{
    DatabaseApiKeyRegistryStore, DatabaseArtifactRepository, DatabaseConnection,
    DatabaseGalleryIndex, DatabaseGenerationDraftRepository, DatabaseGenerationPayloadStore,
    DatabaseJobQueueRepository, DatabasePromptResourceRepository,
    DatabaseResourceCatalogRepository, DatabaseRunHistoryRepository, DatabaseSettingsRepository,
    DatabaseVibeRepository,
};
use atelier_adapter_image_codec::ImageMetadataBlobStore;
use atelier_adapter_keyring::KeyringSecretStore;
use atelier_adapter_novelai::{
    NovelAiClientFactory, NovelAiEmbeddedVibeExtractor, NovelAiSubscriptionProbeClient,
    ReqwestNovelAiClientFactory, ResolverBackedNovelAiAdapter,
};
use atelier_adapter_storage_fs::{
    FileSystemResourceBlobStore, FileSystemResourceContentReader, FileSystemWorkspaceLock,
    FileSystemWorkspaceStore, workspace_database_path,
};
use atelier_app_api::workspace::OpenWorkspaceRequestDto;
use atelier_artifacts::ArtifactService;
use atelier_gallery::GalleryService;
use atelier_jobs::JobQueueRepository;
use atelier_kernel::KernelRuntime;
use atelier_prompt_resources::{PromptChunkService, PromptCompiler, PromptPresetService};
use atelier_resource_catalog::ResourceCatalog;
use atelier_safety::SafetyScanner;
use atelier_secrets::{ApiKeyRegistryService, SecretStore};
use atelier_settings::WorkspaceSettingsService;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use atelier_workspace::{
    WorkspaceLayout, WorkspaceLock, WorkspaceLockLease, WorkspaceRoot, WorkspaceStore,
};
use futures::lock::Mutex;

use crate::events::AppEventHub;
use crate::ports::{
    AppApiKeyService, AppArtifactService, AppGalleryService, AppImageSourceReader, AppKernelPorts,
    AppResourceCatalog, SharedWorkspaceSettings,
};
use crate::usecases::{
    AccountUseCases, DirectorUseCases, EventsUseCases, GalleryUseCases, GenerationUseCases,
    HistoryUseCases, PromptUseCases, ResourceUseCases, SettingsUseCases, VibeUseCases,
    WorkspaceUseCases, generation_history_records_from_queue_snapshot,
};
use crate::{AppResult, error::AppError};

pub struct WorkspaceSession<
    S = KeyringSecretStore,
    F = ReqwestNovelAiClientFactory,
    E = NovelAiEmbeddedVibeExtractor,
> {
    pub(crate) inner: AppInner<S, F, E>,
}

pub struct AppInner<S, F, E> {
    pub root: WorkspaceRoot,
    pub schema_version: u32,
    pub workspace_lock: StdMutex<Box<dyn WorkspaceLockLease>>,
    pub api_keys: AppApiKeyService<S, F>,
    pub settings: WorkspaceSettingsService<DatabaseSettingsRepository>,
    pub generation_drafts:
        atelier_generation::GenerationDraftService<DatabaseGenerationDraftRepository>,
    pub generation_draft_write: Mutex<()>,
    pub settings_state: SharedWorkspaceSettings,
    pub prompt_chunks: PromptChunkService<DatabasePromptResourceRepository>,
    pub prompt_presets: PromptPresetService<DatabasePromptResourceRepository>,
    pub prompt_compiler: PromptCompiler<DatabasePromptResourceRepository>,
    pub artifacts: AppArtifactService,
    pub gallery: AppGalleryService,
    pub gallery_index: DatabaseGalleryIndex,
    pub queue_repository: DatabaseJobQueueRepository,
    pub run_history: DatabaseRunHistoryRepository,
    pub kernel: Mutex<KernelRuntime<AppKernelPorts<S, F, E>>>,
    pub events: AppEventHub,
}

impl
    WorkspaceSession<KeyringSecretStore, ReqwestNovelAiClientFactory, NovelAiEmbeddedVibeExtractor>
{
    /// Opens a workspace with native keyring and `NovelAI` adapters.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// schema validation or keyring setup fails.
    pub async fn open_workspace(request: OpenWorkspaceRequestDto) -> AppResult<Self> {
        Self::open_workspace_with_dependencies(
            request.root,
            KeyringSecretStore::native()?,
            ReqwestNovelAiClientFactory::default(),
        )
        .await
    }
}

impl<S, F> WorkspaceSession<S, F, NovelAiEmbeddedVibeExtractor>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
{
    /// Opens a workspace with injected secret store and `NovelAI` client factory.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// schema validation fails.
    pub async fn open_workspace_with_dependencies(
        root: PathBuf,
        secrets: S,
        factory: F,
    ) -> AppResult<Self> {
        Self::open_workspace_with_dependencies_and_extractor(
            root,
            secrets,
            factory,
            NovelAiEmbeddedVibeExtractor,
        )
        .await
    }

    /// Opens a workspace with injected dependencies and an optional safety scanner.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// schema validation fails.
    pub async fn open_workspace_with_dependencies_and_safety_scanner(
        root: PathBuf,
        secrets: S,
        factory: F,
        safety_scanner: Option<Arc<dyn SafetyScanner>>,
    ) -> AppResult<Self> {
        Self::open_workspace_with_dependencies_and_extractor_and_safety_scanner(
            root,
            secrets,
            factory,
            NovelAiEmbeddedVibeExtractor,
            safety_scanner,
        )
        .await
    }
}

impl<S, F, E> WorkspaceSession<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    /// Opens a workspace with all host-neutral dependencies injected.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// schema validation fails.
    pub async fn open_workspace_with_dependencies_and_extractor(
        root: PathBuf,
        secrets: S,
        factory: F,
        extractor: E,
    ) -> AppResult<Self> {
        Self::open_workspace_with_dependencies_and_extractor_and_safety_scanner(
            root, secrets, factory, extractor, None,
        )
        .await
    }

    /// Opens a workspace with all host-neutral dependencies injected.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// schema validation fails.
    pub async fn open_workspace_with_dependencies_and_extractor_and_safety_scanner(
        root: PathBuf,
        secrets: S,
        factory: F,
        extractor: E,
        safety_scanner: Option<Arc<dyn SafetyScanner>>,
    ) -> AppResult<Self> {
        let root = WorkspaceRoot::new(root);
        let layout = WorkspaceLayout;
        let manifest = FileSystemWorkspaceStore::new()
            .initialize(&root, &layout)
            .await?;
        let lease = FileSystemWorkspaceLock::new()
            .acquire(&root, &layout)
            .await?;
        let connection = DatabaseConnection::open(workspace_database_path(&root))?;
        let api_key_store = DatabaseApiKeyRegistryStore::new(connection.clone());
        let queue_repository = DatabaseJobQueueRepository::new(connection.clone());
        let run_history = DatabaseRunHistoryRepository::new(connection.clone());
        let api_keys = ApiKeyRegistryService::new(
            api_key_store,
            secrets,
            NovelAiSubscriptionProbeClient::new(factory.clone()),
        );
        let resource_repository = DatabaseResourceCatalogRepository::new(connection.clone());
        let prompt_repository = DatabasePromptResourceRepository::new(connection.clone());
        let settings_repository = DatabaseSettingsRepository::new(connection.clone());
        let settings = WorkspaceSettingsService::new(settings_repository);
        let generation_drafts = atelier_generation::GenerationDraftService::new(
            DatabaseGenerationDraftRepository::new(connection.clone()),
        );
        let settings_state = SharedWorkspaceSettings::new(settings.get_workspace_settings().await?);
        let blob_store = FileSystemResourceBlobStore::new(root.clone(), layout);
        let resource_reader =
            FileSystemResourceContentReader::new(resource_repository.clone(), blob_store.clone());
        let variant_builder = atelier_adapter_image_codec::ImageCodecVariantBuilder::new(
            AppImageSourceReader::new(resource_reader.clone()),
            settings_state.clone(),
        );
        let resources: AppResourceCatalog = ResourceCatalog::new(
            resource_repository.clone(),
            ImageMetadataBlobStore::new(blob_store),
            variant_builder,
        );
        let artifacts: AppArtifactService = ArtifactService::new(
            DatabaseArtifactRepository::new(connection.clone()),
            resource_repository,
        );
        let gallery_index = DatabaseGalleryIndex::new(connection.clone());
        let gallery = GalleryService::new(gallery_index.clone());
        let events = AppEventHub::default();
        let prompt_compiler = PromptCompiler::new(prompt_repository.clone());
        let ports = AppKernelPorts {
            payloads: DatabaseGenerationPayloadStore::new(connection.clone()),
            prompt_compiler: PromptCompiler::new(prompt_repository.clone()),
            novelai: ResolverBackedNovelAiAdapter::new(api_keys.clone(), factory),
            resources,
            artifacts: artifacts.clone(),
            gallery: gallery.clone(),
            resource_reader,
            vibes: DatabaseVibeRepository::new(connection),
            extractor,
            events: events.clone(),
            settings_state: settings_state.clone(),
            safety_scanner,
        };
        let restored_snapshot = queue_repository
            .load_queue_snapshot()
            .await
            .map_err(|error| AppError::new("job_queue", error.to_string()))?;
        let kernel = if let Some(snapshot) = restored_snapshot {
            let runtime = KernelRuntime::from_recovered_queue_snapshot(ports, snapshot)
                .map_err(AppError::from)?;
            let snapshot = runtime.queue_snapshot();
            let history =
                generation_history_records_from_queue_snapshot(&run_history, &snapshot).await?;
            queue_repository
                .commit_queue_and_history(Some(&snapshot), history)
                .map_err(|error| AppError::new("job_queue", error.to_string()))?;
            runtime
        } else {
            KernelRuntime::new(ports)
        };

        Ok(Self {
            inner: AppInner {
                root,
                schema_version: manifest.schema_version,
                workspace_lock: StdMutex::new(lease),
                api_keys,
                settings,
                generation_drafts,
                generation_draft_write: Mutex::new(()),
                settings_state,
                prompt_chunks: PromptChunkService::new(prompt_repository.clone()),
                prompt_presets: PromptPresetService::new(prompt_repository),
                prompt_compiler,
                artifacts,
                gallery,
                gallery_index,
                queue_repository,
                run_history,
                kernel: Mutex::new(kernel),
                events,
            },
        })
    }
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
        let mut lease = self.inner.workspace_lock.lock().map_err(|_| {
            AppError::new(
                "workspace_lock_state_poisoned",
                "workspace lock state is unavailable",
            )
        })?;
        lease.release().map_err(AppError::from)
    }

    #[must_use]
    pub const fn workspace(&self) -> WorkspaceUseCases<'_, S, F, E> {
        WorkspaceUseCases { app: self }
    }

    #[must_use]
    pub const fn account(&self) -> AccountUseCases<'_, S, F, E> {
        AccountUseCases { app: self }
    }

    #[must_use]
    pub const fn prompt(&self) -> PromptUseCases<'_, S, F, E> {
        PromptUseCases { app: self }
    }

    #[must_use]
    pub const fn resources(&self) -> ResourceUseCases<'_, S, F, E> {
        ResourceUseCases { app: self }
    }

    #[must_use]
    pub const fn settings(&self) -> SettingsUseCases<'_, S, F, E> {
        SettingsUseCases { app: self }
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
    pub const fn gallery(&self) -> GalleryUseCases<'_, S, F, E> {
        GalleryUseCases { app: self }
    }

    #[must_use]
    pub const fn history(&self) -> HistoryUseCases<'_, S, F, E> {
        HistoryUseCases { app: self }
    }

    #[must_use]
    pub const fn events(&self) -> EventsUseCases<'_, S, F, E> {
        EventsUseCases { app: self }
    }
}
