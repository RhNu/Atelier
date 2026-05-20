use std::path::PathBuf;
use std::sync::Mutex as StdMutex;

use futures::lock::Mutex;
use nai_atelier_adapter_database::{
    DatabaseApiKeyRegistryStore, DatabaseArtifactRepository, DatabaseConnection,
    DatabaseGalleryIndex, DatabaseGenerationPayloadStore, DatabasePromptResourceRepository,
    DatabaseResourceCatalogRepository, DatabaseVibeRepository,
};
use nai_atelier_adapter_keyring::KeyringSecretStore;
use nai_atelier_adapter_novelai::{
    NovelAiClientFactory, NovelAiEmbeddedVibeExtractor, NovelAiSubscriptionProbeClient,
    ReqwestNovelAiClientFactory, ResolverBackedNovelAiAdapter,
};
use nai_atelier_adapter_storage_fs::{
    FileSystemResourceBlobStore, FileSystemResourceContentReader, FileSystemWorkspaceLock,
    FileSystemWorkspaceStore, workspace_database_path,
};
use nai_atelier_app_api::workspace::OpenWorkspaceRequestDto;
use nai_atelier_artifacts::ArtifactService;
use nai_atelier_gallery::GalleryService;
use nai_atelier_kernel::KernelRuntime;
use nai_atelier_prompt_lexicon::PromptLexicon;
use nai_atelier_prompt_resources::{PromptChunkService, PromptCompiler};
use nai_atelier_resource_catalog::ResourceCatalog;
use nai_atelier_secrets::{ApiKeyRegistryService, SecretStore};
use nai_atelier_vibe::EmbeddedVibeDocumentExtractor;
use nai_atelier_workspace::{
    WorkspaceLayout, WorkspaceLock, WorkspaceLockLease, WorkspaceLockRequest, WorkspaceRoot,
    WorkspaceStore,
};

use crate::events::AppEventHub;
use crate::ports::{
    AppApiKeyService, AppArtifactService, AppGalleryService, AppKernelPorts, AppResourceCatalog,
    NoopVariantBuilder,
};
use crate::usecases::{
    AccountUseCases, EventsUseCases, GalleryUseCases, GenerationUseCases, PromptUseCases,
    VibeUseCases, WorkspaceUseCases,
};
use crate::{AppResult, error::AppError};

pub struct AtelierApp<
    S = KeyringSecretStore,
    F = ReqwestNovelAiClientFactory,
    E = NovelAiEmbeddedVibeExtractor,
> {
    pub(crate) inner: AppInner<S, F, E>,
}

pub struct AppInner<S, F, E> {
    pub root: WorkspaceRoot,
    pub schema_version: u32,
    pub _lease: StdMutex<Box<dyn WorkspaceLockLease>>,
    pub api_keys: AppApiKeyService<S, F>,
    pub prompt_chunks: PromptChunkService<DatabasePromptResourceRepository>,
    pub prompt_compiler: PromptCompiler<DatabasePromptResourceRepository>,
    pub lexicon: PromptLexicon,
    pub gallery: AppGalleryService,
    pub kernel: Mutex<KernelRuntime<AppKernelPorts<S, F, E>>>,
    pub events: AppEventHub,
}

impl AtelierApp<KeyringSecretStore, ReqwestNovelAiClientFactory, NovelAiEmbeddedVibeExtractor> {
    /// Opens a workspace with native keyring and `NovelAI` adapters.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// migrations, keyring setup, or embedded lexicon loading fails.
    pub async fn open_workspace(request: OpenWorkspaceRequestDto) -> AppResult<Self> {
        Self::open_workspace_with_dependencies(
            request.root,
            KeyringSecretStore::native()?,
            ReqwestNovelAiClientFactory::default(),
        )
        .await
    }
}

impl<S, F> AtelierApp<S, F, NovelAiEmbeddedVibeExtractor>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
{
    /// Opens a workspace with injected secret store and `NovelAI` client factory.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// migrations, or embedded lexicon loading fails.
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
}

impl<S, F, E> AtelierApp<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    /// Opens a workspace with all host-neutral dependencies injected.
    ///
    /// # Errors
    /// Returns an error when workspace initialization, locking, database
    /// migrations, or embedded lexicon loading fails.
    pub async fn open_workspace_with_dependencies_and_extractor(
        root: PathBuf,
        secrets: S,
        factory: F,
        extractor: E,
    ) -> AppResult<Self> {
        let root = WorkspaceRoot::new(root);
        let layout = WorkspaceLayout;
        let manifest = FileSystemWorkspaceStore::new()
            .initialize(&root, &layout)
            .await?;
        let lease = FileSystemWorkspaceLock::new()
            .acquire(&root, &layout, WorkspaceLockRequest::new("nai-atelier-app"))
            .await?;
        let connection = DatabaseConnection::open(workspace_database_path(&root))?;
        let api_key_store = DatabaseApiKeyRegistryStore::new(connection.clone());
        let api_keys = ApiKeyRegistryService::new(
            api_key_store,
            secrets,
            NovelAiSubscriptionProbeClient::new(factory.clone()),
        );
        let resource_repository = DatabaseResourceCatalogRepository::new(connection.clone());
        let prompt_repository = DatabasePromptResourceRepository::new(connection.clone());
        let blob_store = FileSystemResourceBlobStore::new(root.clone(), layout);
        let resource_reader =
            FileSystemResourceContentReader::new(resource_repository.clone(), blob_store.clone());
        let resources: AppResourceCatalog =
            ResourceCatalog::new(resource_repository.clone(), blob_store, NoopVariantBuilder);
        let artifacts: AppArtifactService = ArtifactService::new(
            DatabaseArtifactRepository::new(connection.clone()),
            resource_repository,
        );
        let gallery = GalleryService::new(DatabaseGalleryIndex::new(connection.clone()));
        let events = AppEventHub::default();
        let prompt_compiler = PromptCompiler::new(prompt_repository.clone());
        let ports = AppKernelPorts {
            payloads: DatabaseGenerationPayloadStore::new(connection.clone()),
            prompt_compiler: PromptCompiler::new(prompt_repository.clone()),
            novelai: ResolverBackedNovelAiAdapter::new(api_keys.clone(), factory),
            resources,
            artifacts,
            gallery: gallery.clone(),
            resource_reader,
            vibes: DatabaseVibeRepository::new(connection),
            extractor,
            events: events.clone(),
        };

        Ok(Self {
            inner: AppInner {
                root,
                schema_version: manifest.schema_version,
                _lease: StdMutex::new(lease),
                api_keys,
                prompt_chunks: PromptChunkService::new(prompt_repository),
                prompt_compiler,
                lexicon: PromptLexicon::load_embedded().map_err(AppError::from)?,
                gallery,
                kernel: Mutex::new(KernelRuntime::new(ports)),
                events,
            },
        })
    }
}

impl<S, F, E> AtelierApp<S, F, E> {
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
    pub const fn generation(&self) -> GenerationUseCases<'_, S, F, E> {
        GenerationUseCases { app: self }
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
    pub const fn events(&self) -> EventsUseCases<'_, S, F, E> {
        EventsUseCases { app: self }
    }
}
