use atelier_adapter_database::DatabaseResourceLibraryRepository;
use atelier_app_api::resource_library::{
    DeleteLibraryFolderRequestDto, DeleteLibraryFolderResponseDto, GetLibrarySnapshotRequestDto,
    LibraryFolderDto, LibraryNamespaceDto, LibraryResourceDto, LibrarySnapshotDto,
    UpdateLibraryResourceRequestDto, UpsertLibraryFolderRequestDto,
};
use atelier_resource_library::{
    LibraryFolderId, LibraryNamespace, LibraryResourceId, ResourceIdentifier,
    ResourceLibraryService, UpdateLibraryResourceRequest, UpsertLibraryFolderRequest,
};

use crate::AppResult;

pub struct ResourceLibraryUseCases<'a> {
    pub(crate) prompt_resource_write: &'a futures::lock::Mutex<()>,
    pub(crate) library: &'a ResourceLibraryService<DatabaseResourceLibraryRepository>,
}

impl ResourceLibraryUseCases<'_> {
    pub async fn snapshot(
        &self,
        request: GetLibrarySnapshotRequestDto,
    ) -> AppResult<LibrarySnapshotDto> {
        let snapshot = self
            .library
            .snapshot(namespace_to_domain(request.namespace))
            .await?;
        Ok(LibrarySnapshotDto {
            folders: snapshot
                .folders
                .into_iter()
                .map(|entry| LibraryFolderDto {
                    folder_id: entry.folder.id.as_str().to_owned(),
                    namespace: namespace_to_dto(entry.folder.namespace),
                    parent_id: entry
                        .folder
                        .parent_id
                        .as_ref()
                        .map(|id| id.as_str().to_owned()),
                    identifier: entry.folder.identifier.as_str().to_owned(),
                    display_name: entry.folder.display_name,
                    path: entry.path.as_str().to_owned(),
                    created_at_ms: entry.folder.created_at_ms,
                    updated_at_ms: entry.folder.updated_at_ms,
                })
                .collect(),
            resources: snapshot
                .resources
                .into_iter()
                .map(|entry| LibraryResourceDto {
                    resource_id: entry.resource.id.as_str().to_owned(),
                    namespace: namespace_to_dto(entry.resource.namespace),
                    folder_id: entry
                        .resource
                        .folder_id
                        .as_ref()
                        .map(|id| id.as_str().to_owned()),
                    identifier: entry.resource.name.identifier.as_str().to_owned(),
                    display_name: entry.resource.name.display_name,
                    aliases: entry.resource.name.aliases,
                    path: entry.path.as_str().to_owned(),
                    created_at_ms: entry.resource.created_at_ms,
                    updated_at_ms: entry.resource.updated_at_ms,
                })
                .collect(),
        })
    }

    pub async fn upsert_folder(
        &self,
        request: UpsertLibraryFolderRequestDto,
    ) -> AppResult<LibraryFolderDto> {
        let _guard = self.prompt_resource_write.lock().await;
        let namespace = namespace_to_domain(request.namespace);
        let saved = self
            .library
            .upsert_folder(UpsertLibraryFolderRequest {
                folder_id: request
                    .folder_id
                    .as_deref()
                    .map(LibraryFolderId::parse)
                    .transpose()?,
                namespace,
                parent_id: request
                    .parent_id
                    .as_deref()
                    .map(LibraryFolderId::parse)
                    .transpose()?,
                identifier: ResourceIdentifier::parse(&request.identifier)?,
                display_name: request.display_name,
            })
            .await?;
        let snapshot = self.library.snapshot(namespace).await?;
        let entry = snapshot
            .folders
            .into_iter()
            .find(|entry| entry.folder.id == saved.id)
            .expect("saved folder appears in snapshot");
        Ok(LibraryFolderDto {
            folder_id: entry.folder.id.as_str().to_owned(),
            namespace: namespace_to_dto(entry.folder.namespace),
            parent_id: entry
                .folder
                .parent_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            identifier: entry.folder.identifier.as_str().to_owned(),
            display_name: entry.folder.display_name,
            path: entry.path.as_str().to_owned(),
            created_at_ms: entry.folder.created_at_ms,
            updated_at_ms: entry.folder.updated_at_ms,
        })
    }

    pub async fn update_resource(
        &self,
        request: UpdateLibraryResourceRequestDto,
    ) -> AppResult<LibraryResourceDto> {
        let _guard = self.prompt_resource_write.lock().await;
        let resource = self
            .library
            .update_resource(UpdateLibraryResourceRequest {
                resource_id: LibraryResourceId::parse(&request.resource_id)?,
                folder_id: request
                    .folder_id
                    .as_deref()
                    .map(LibraryFolderId::parse)
                    .transpose()?,
                identifier: ResourceIdentifier::parse(&request.identifier)?,
                display_name: request.display_name,
                aliases: request.aliases,
            })
            .await?;
        let snapshot = self.library.snapshot(resource.namespace).await?;
        let entry = snapshot
            .resources
            .into_iter()
            .find(|entry| entry.resource.id == resource.id)
            .expect("saved resource appears in snapshot");
        Ok(LibraryResourceDto {
            resource_id: entry.resource.id.as_str().to_owned(),
            namespace: namespace_to_dto(entry.resource.namespace),
            folder_id: entry
                .resource
                .folder_id
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            identifier: entry.resource.name.identifier.as_str().to_owned(),
            display_name: entry.resource.name.display_name,
            aliases: entry.resource.name.aliases,
            path: entry.path.as_str().to_owned(),
            created_at_ms: entry.resource.created_at_ms,
            updated_at_ms: entry.resource.updated_at_ms,
        })
    }

    pub async fn delete_folder(
        &self,
        request: DeleteLibraryFolderRequestDto,
    ) -> AppResult<DeleteLibraryFolderResponseDto> {
        let _guard = self.prompt_resource_write.lock().await;
        let id = LibraryFolderId::parse(&request.folder_id)?;
        self.library.delete_folder(&id).await?;
        Ok(DeleteLibraryFolderResponseDto { deleted: true })
    }
}

const fn namespace_to_domain(value: LibraryNamespaceDto) -> LibraryNamespace {
    match value {
        LibraryNamespaceDto::PromptChunk => LibraryNamespace::PromptChunk,
        LibraryNamespaceDto::MainPreset => LibraryNamespace::MainPreset,
        LibraryNamespaceDto::CharacterPreset => LibraryNamespace::CharacterPreset,
        LibraryNamespaceDto::Vibe => LibraryNamespace::Vibe,
    }
}

const fn namespace_to_dto(value: LibraryNamespace) -> LibraryNamespaceDto {
    match value {
        LibraryNamespace::PromptChunk => LibraryNamespaceDto::PromptChunk,
        LibraryNamespace::MainPreset => LibraryNamespaceDto::MainPreset,
        LibraryNamespace::CharacterPreset => LibraryNamespaceDto::CharacterPreset,
        LibraryNamespace::Vibe => LibraryNamespaceDto::Vibe,
    }
}
