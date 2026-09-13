use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    LibraryTree, ResourceIdentifier, ResourceLibraryError, ResourceLibraryErrorKind,
    ResourceLibraryRepository, ResourceLibraryResult, ResourceName, ResourcePath,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryFolderEntry {
    pub folder: LibraryFolder,
    pub path: ResourcePath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryResourceEntry {
    pub resource: LibraryResource,
    pub path: ResourcePath,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibrarySnapshot {
    pub folders: Vec<LibraryFolderEntry>,
    pub resources: Vec<LibraryResourceEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpsertLibraryFolderRequest {
    pub folder_id: Option<LibraryFolderId>,
    pub namespace: LibraryNamespace,
    pub parent_id: Option<LibraryFolderId>,
    pub identifier: ResourceIdentifier,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateLibraryResourceRequest {
    pub resource_id: LibraryResourceId,
    pub folder_id: Option<LibraryFolderId>,
    pub identifier: ResourceIdentifier,
    pub display_name: String,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ResourceLibraryService<R> {
    repository: R,
}

impl<R> ResourceLibraryService<R>
where
    R: ResourceLibraryRepository,
{
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Loads a namespace as path-resolved folder and resource entries.
    ///
    /// # Errors
    /// Returns an error when repository data is unavailable or forms an invalid tree.
    pub async fn snapshot(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<LibrarySnapshot> {
        let (tree, folders, resources) = self.load_tree(namespace).await?;
        let mut folder_entries = folders
            .into_iter()
            .map(|folder| {
                let path = tree.folder_path(&folder.id)?;
                Ok(LibraryFolderEntry { folder, path })
            })
            .collect::<ResourceLibraryResult<Vec<_>>>()?;
        let mut resource_entries = resources
            .into_iter()
            .map(|resource| {
                let path = tree.resource_path(&resource.id)?;
                Ok(LibraryResourceEntry { resource, path })
            })
            .collect::<ResourceLibraryResult<Vec<_>>>()?;
        folder_entries.sort_by(|left, right| left.path.cmp(&right.path));
        resource_entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(LibrarySnapshot {
            folders: folder_entries,
            resources: resource_entries,
        })
    }

    /// Creates or updates one folder.
    ///
    /// # Errors
    /// Returns an error for invalid ancestry, conflicts, or persistence failures.
    pub async fn upsert_folder(
        &self,
        request: UpsertLibraryFolderRequest,
    ) -> ResourceLibraryResult<LibraryFolder> {
        let (mut tree, folders, _) = self.load_tree(request.namespace).await?;
        let now = unix_ms();
        let existing = request
            .folder_id
            .as_ref()
            .and_then(|id| folders.iter().find(|folder| &folder.id == id).cloned());
        if request.folder_id.is_some() && existing.is_none() {
            return Err(not_found("folder"));
        }
        let id = request.folder_id.unwrap_or_else(LibraryFolderId::allocate);
        if existing.is_some() {
            tree.move_folder(&id, request.parent_id.clone(), now)?;
        }
        let mut folder = LibraryFolder::new(
            id,
            request.namespace,
            request.parent_id,
            request.identifier,
            &request.display_name,
            now,
        )?;
        if let Some(existing) = existing {
            folder.created_at_ms = existing.created_at_ms;
        }
        tree.insert_folder(folder.clone())?;
        self.repository.save_folder(folder.clone()).await?;
        Ok(folder)
    }

    /// Moves or renames one resource while preserving its stable id.
    ///
    /// # Errors
    /// Returns an error for invalid metadata, conflicts, or persistence failures.
    pub async fn update_resource(
        &self,
        request: UpdateLibraryResourceRequest,
    ) -> ResourceLibraryResult<LibraryResource> {
        let existing = self
            .repository
            .get_resource(&request.resource_id)
            .await?
            .ok_or_else(|| not_found("resource"))?;
        let (mut tree, _, _) = self.load_tree(existing.namespace).await?;
        let now = unix_ms();
        tree.move_resource(&existing.id, request.folder_id.clone(), now)?;
        let resource = LibraryResource {
            id: existing.id,
            namespace: existing.namespace,
            folder_id: request.folder_id,
            name: ResourceName::new(request.identifier, &request.display_name, request.aliases)?,
            created_at_ms: existing.created_at_ms,
            updated_at_ms: now,
        };
        tree.insert_resource(resource.clone())?;
        self.repository.save_resource(resource.clone()).await?;
        Ok(resource)
    }

    /// Deletes a folder using repository-defined cascade and reference safety.
    ///
    /// # Errors
    /// Returns an error when the folder is missing, referenced, or cannot be deleted.
    pub async fn delete_folder(&self, id: &LibraryFolderId) -> ResourceLibraryResult<()> {
        self.repository.delete_folder(id).await
    }

    async fn load_tree(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<(LibraryTree, Vec<LibraryFolder>, Vec<LibraryResource>)> {
        let folders = self.repository.list_folders(namespace).await?;
        let resources = self.repository.list_resources(namespace).await?;
        let mut tree = LibraryTree::default();
        let mut pending = folders.clone();
        while !pending.is_empty() {
            let before = pending.len();
            pending.retain(|folder| tree.insert_folder(folder.clone()).is_err());
            if pending.len() == before {
                return Err(ResourceLibraryError::new(
                    ResourceLibraryErrorKind::Cycle,
                    "library folders contain an invalid ancestry cycle",
                ));
            }
        }
        for resource in &resources {
            tree.insert_resource(resource.clone())?;
        }
        Ok((tree, folders, resources))
    }
}

fn not_found(subject: &str) -> ResourceLibraryError {
    ResourceLibraryError::new(
        ResourceLibraryErrorKind::NotFound,
        format!("library {subject} does not exist"),
    )
}

fn unix_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
