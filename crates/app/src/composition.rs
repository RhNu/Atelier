use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};

use atelier_adapter_database::{
    DatabaseArtifactRepository, DatabaseConnection, DatabaseGalleryIndex,
    DatabaseGenerationDraftRepository, DatabaseGenerationPayloadStore, DatabaseGenerationStore,
    DatabasePromptResourceRepository, DatabaseResourceCatalogRepository,
    DatabaseResourceLibraryRepository, DatabaseRunHistoryRepository, DatabaseSettingsRepository,
    DatabaseVibeRepository,
};
use atelier_adapter_image_codec::ImageMetadataBlobStore;
use atelier_adapter_novelai::{NovelAiClientFactory, ResolverBackedNovelAiAdapter};
use atelier_adapter_storage_fs::{
    FileSystemResourceBlobStore, FileSystemResourceContentReader, FileSystemWorkspaceLock,
    FileSystemWorkspaceStore, workspace_database_path,
};
use atelier_artifacts::ArtifactService;
use atelier_gallery::GalleryService;
use atelier_jobs::GenerationStore;
use atelier_kernel::KernelRuntime;
use atelier_prompt_resources::{PromptChunkService, PromptCompiler, PromptPresetService};
use atelier_resource_catalog::ResourceCatalog;
use atelier_resource_library::ResourceLibraryService;
use atelier_safety::SafetyScanner;
use atelier_secrets::SecretStore;
use atelier_settings::WorkspaceSettingsService;
use atelier_vibe::EmbeddedVibeDocumentExtractor;
use atelier_workspace::{WorkspaceLayout, WorkspaceLock, WorkspaceRoot, WorkspaceStore};
use futures::lock::Mutex;

use crate::events::AppEventHub;
use crate::ports::{
    AppApiKeyService, AppArtifactService, AppImageSourceReader, AppKernelPorts, AppResourceCatalog,
    SharedWorkspaceSettings,
};
use crate::usecases::generation_history_records_from_queue_snapshot;
use crate::{AppResult, error::AppError};

pub struct WorkspaceServices<S, F, E> {
    pub api_keys: AppApiKeyService<S, F>,
    pub factory: F,
    pub extractor: E,
    pub safety_scanner: Option<Arc<dyn SafetyScanner>>,
}

impl<S, F, E> WorkspaceSession<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    pub(crate) async fn open(
        root: PathBuf,
        services: WorkspaceServices<S, F, E>,
    ) -> AppResult<Self> {
        let WorkspaceServices {
            api_keys,
            factory,
            extractor,
            safety_scanner,
        } = services;
        let root = WorkspaceRoot::new(root);
        let layout = WorkspaceLayout;
        let manifest = FileSystemWorkspaceStore::new()
            .initialize(&root, &layout)
            .await?;
        let lease = FileSystemWorkspaceLock::new()
            .acquire(&root, &layout)
            .await?;
        let connection = DatabaseConnection::open(workspace_database_path(&root))?;
        let queue_repository = DatabaseGenerationStore::new(connection.clone());
        let run_history = DatabaseRunHistoryRepository::new(connection.clone());
        let resource_repository = DatabaseResourceCatalogRepository::new(connection.clone());
        let prompt_repository = DatabasePromptResourceRepository::new(connection.clone());
        let resource_library =
            ResourceLibraryService::new(DatabaseResourceLibraryRepository::new(connection.clone()));
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
            run_history: run_history.clone(),
            payloads: DatabaseGenerationPayloadStore::new(connection.clone()),
            prompt_compiler: PromptCompiler::new(prompt_repository.clone()),
            novelai: ResolverBackedNovelAiAdapter::new(api_keys.clone(), factory),
            resources: resources.clone(),
            artifacts: artifacts.clone(),
            gallery: gallery.clone(),
            resource_reader: resource_reader.clone(),
            vibes: DatabaseVibeRepository::new(connection),
            extractor,
            events: events.clone(),
            settings_state: settings_state.clone(),
            safety_scanner: safety_scanner.clone(),
        };
        let kernel = Self::restore_kernel(ports, &queue_repository, &run_history).await?;

        Ok(Self {
            root,
            schema_version: manifest.schema_version,
            workspace_lock: StdMutex::new(lease),
            api_keys,
            settings,
            generation_drafts,
            generation_draft_write: Mutex::new(()),
            gallery_safety_rescan: Mutex::new(()),
            settings_state,
            prompt_chunks: PromptChunkService::new(prompt_repository.clone()),
            prompt_presets: PromptPresetService::new(prompt_repository),
            prompt_compiler,
            prompt_resource_write: Mutex::new(()),
            resource_library,
            artifacts,
            gallery,
            gallery_index,
            resources,
            resource_reader,
            safety_scanner,
            queue_repository,
            run_history,
            queue: kernel.queue_view(),
            workflows: kernel.context().clone(),
            kernel: Mutex::new(kernel),
            events,
        })
    }
    async fn restore_kernel(
        ports: AppKernelPorts<S, F, E>,
        queue_repository: &DatabaseGenerationStore,
        run_history: &DatabaseRunHistoryRepository,
    ) -> AppResult<KernelRuntime<AppKernelPorts<S, F, E>>> {
        let restored_snapshot = queue_repository
            .load()
            .await
            .map_err(|error| AppError::new("job_queue", error.to_string()))?;
        let kernel = if let Some(snapshot) = restored_snapshot {
            let runtime = KernelRuntime::from_recovered_queue_snapshot(ports, snapshot)
                .map_err(AppError::from)?;
            let snapshot = runtime.queue_snapshot();
            let history =
                generation_history_records_from_queue_snapshot(run_history, &snapshot).await?;
            queue_repository
                .commit(Some(&snapshot), history)
                .await
                .map_err(|error| AppError::new("job_queue", error.to_string()))?;
            runtime
        } else {
            KernelRuntime::new(ports)
        };

        Ok(kernel)
    }
}

use crate::session::WorkspaceSession;
