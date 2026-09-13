use std::collections::{BTreeMap, BTreeSet};

use crate::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    ResourceIdentifier, ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryResult,
    ResourcePath,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryNodeRef {
    Folder(LibraryFolderId),
    Resource(LibraryResourceId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryChildren {
    pub folders: Vec<LibraryFolder>,
    pub resources: Vec<LibraryResource>,
}

#[derive(Clone, Debug, Default)]
pub struct LibraryTree {
    folders: BTreeMap<LibraryFolderId, LibraryFolder>,
    resources: BTreeMap<LibraryResourceId, LibraryResource>,
}

impl LibraryTree {
    pub fn insert_folder(&mut self, folder: LibraryFolder) -> ResourceLibraryResult<()> {
        self.validate_parent(folder.namespace, folder.parent_id.as_ref())?;
        self.ensure_name_available(
            folder.namespace,
            folder.parent_id.as_ref(),
            &folder.identifier,
            Some(&LibraryNodeRef::Folder(folder.id.clone())),
        )?;
        self.folders.insert(folder.id.clone(), folder);
        Ok(())
    }

    pub fn insert_resource(&mut self, resource: LibraryResource) -> ResourceLibraryResult<()> {
        self.validate_parent(resource.namespace, resource.folder_id.as_ref())?;
        self.ensure_name_available(
            resource.namespace,
            resource.folder_id.as_ref(),
            &resource.name.identifier,
            Some(&LibraryNodeRef::Resource(resource.id.clone())),
        )?;
        self.resources.insert(resource.id.clone(), resource);
        Ok(())
    }

    pub fn move_folder(
        &mut self,
        id: &LibraryFolderId,
        destination: Option<LibraryFolderId>,
        now_ms: u64,
    ) -> ResourceLibraryResult<()> {
        let folder = self.require_folder(id)?.clone();
        self.validate_parent(folder.namespace, destination.as_ref())?;
        if destination
            .as_ref()
            .is_some_and(|parent| parent == id || self.folder_ancestors(parent).contains(id))
        {
            return Err(ResourceLibraryError::new(
                ResourceLibraryErrorKind::Cycle,
                "folder cannot be moved into itself or one of its descendants",
            ));
        }
        self.ensure_name_available(
            folder.namespace,
            destination.as_ref(),
            &folder.identifier,
            Some(&LibraryNodeRef::Folder(id.clone())),
        )?;
        let target = self.folders.get_mut(id).expect("validated folder");
        target.parent_id = destination;
        target.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn move_resource(
        &mut self,
        id: &LibraryResourceId,
        destination: Option<LibraryFolderId>,
        now_ms: u64,
    ) -> ResourceLibraryResult<()> {
        let resource = self.require_resource(id)?.clone();
        self.validate_parent(resource.namespace, destination.as_ref())?;
        self.ensure_name_available(
            resource.namespace,
            destination.as_ref(),
            &resource.name.identifier,
            Some(&LibraryNodeRef::Resource(id.clone())),
        )?;
        let target = self.resources.get_mut(id).expect("validated resource");
        target.folder_id = destination;
        target.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn children(
        &self,
        namespace: LibraryNamespace,
        parent_id: Option<&LibraryFolderId>,
    ) -> ResourceLibraryResult<LibraryChildren> {
        self.validate_parent(namespace, parent_id)?;
        let mut folders = self
            .folders
            .values()
            .filter(|folder| {
                folder.namespace == namespace && folder.parent_id.as_ref() == parent_id
            })
            .cloned()
            .collect::<Vec<_>>();
        let mut resources = self
            .resources
            .values()
            .filter(|resource| {
                resource.namespace == namespace && resource.folder_id.as_ref() == parent_id
            })
            .cloned()
            .collect::<Vec<_>>();
        folders.sort_by(|left, right| left.identifier.cmp(&right.identifier));
        resources.sort_by(|left, right| left.name.identifier.cmp(&right.name.identifier));
        Ok(LibraryChildren { folders, resources })
    }

    pub fn resource_path(&self, id: &LibraryResourceId) -> ResourceLibraryResult<ResourcePath> {
        let resource = self.require_resource(id)?;
        let mut segments = self.folder_segments(resource.folder_id.as_ref())?;
        segments.push(&resource.name.identifier);
        Ok(ResourcePath::from_segments(segments))
    }

    pub fn descendant_resources(
        &self,
        id: &LibraryFolderId,
    ) -> ResourceLibraryResult<Vec<LibraryResourceId>> {
        let folder = self.require_folder(id)?;
        let descendants = self.descendant_folder_ids(id);
        Ok(self
            .resources
            .values()
            .filter(|resource| {
                resource.namespace == folder.namespace
                    && resource
                        .folder_id
                        .as_ref()
                        .is_some_and(|parent| parent == id || descendants.contains(parent))
            })
            .map(|resource| resource.id.clone())
            .collect())
    }

    fn require_folder(&self, id: &LibraryFolderId) -> ResourceLibraryResult<&LibraryFolder> {
        self.folders.get(id).ok_or_else(|| not_found("folder"))
    }

    fn require_resource(&self, id: &LibraryResourceId) -> ResourceLibraryResult<&LibraryResource> {
        self.resources.get(id).ok_or_else(|| not_found("resource"))
    }

    fn validate_parent(
        &self,
        namespace: LibraryNamespace,
        parent_id: Option<&LibraryFolderId>,
    ) -> ResourceLibraryResult<()> {
        let Some(parent_id) = parent_id else {
            return Ok(());
        };
        let parent = self.require_folder(parent_id)?;
        if parent.namespace != namespace {
            return Err(ResourceLibraryError::new(
                ResourceLibraryErrorKind::Conflict,
                "folder and child must belong to the same namespace",
            ));
        }
        Ok(())
    }

    fn ensure_name_available(
        &self,
        namespace: LibraryNamespace,
        parent_id: Option<&LibraryFolderId>,
        identifier: &ResourceIdentifier,
        except: Option<&LibraryNodeRef>,
    ) -> ResourceLibraryResult<()> {
        let folder_conflict = self.folders.values().any(|folder| {
            folder.namespace == namespace
                && folder.parent_id.as_ref() == parent_id
                && &folder.identifier == identifier
                && except != Some(&LibraryNodeRef::Folder(folder.id.clone()))
        });
        let resource_conflict = self.resources.values().any(|resource| {
            resource.namespace == namespace
                && resource.folder_id.as_ref() == parent_id
                && &resource.name.identifier == identifier
                && except != Some(&LibraryNodeRef::Resource(resource.id.clone()))
        });
        if folder_conflict || resource_conflict {
            return Err(ResourceLibraryError::new(
                ResourceLibraryErrorKind::Conflict,
                format!(
                    "identifier `{}` is already used in this folder",
                    identifier.as_str()
                ),
            ));
        }
        Ok(())
    }

    fn folder_ancestors(&self, id: &LibraryFolderId) -> BTreeSet<LibraryFolderId> {
        let mut ancestors = BTreeSet::new();
        let mut cursor = Some(id);
        while let Some(current) = cursor {
            if !ancestors.insert(current.clone()) {
                break;
            }
            cursor = self
                .folders
                .get(current)
                .and_then(|folder| folder.parent_id.as_ref());
        }
        ancestors
    }

    fn descendant_folder_ids(&self, id: &LibraryFolderId) -> BTreeSet<LibraryFolderId> {
        let mut descendants = BTreeSet::new();
        let mut pending = vec![id.clone()];
        while let Some(parent) = pending.pop() {
            for child in self
                .folders
                .values()
                .filter(|folder| folder.parent_id.as_ref() == Some(&parent))
            {
                if descendants.insert(child.id.clone()) {
                    pending.push(child.id.clone());
                }
            }
        }
        descendants
    }

    fn folder_segments<'a>(
        &'a self,
        folder_id: Option<&LibraryFolderId>,
    ) -> ResourceLibraryResult<Vec<&'a ResourceIdentifier>> {
        let mut segments = Vec::new();
        let mut cursor = folder_id;
        let mut visited = BTreeSet::new();
        while let Some(id) = cursor {
            if !visited.insert(id.clone()) {
                return Err(ResourceLibraryError::new(
                    ResourceLibraryErrorKind::Cycle,
                    "folder ancestry contains a cycle",
                ));
            }
            let folder = self.require_folder(id)?;
            segments.push(&folder.identifier);
            cursor = folder.parent_id.as_ref();
        }
        segments.reverse();
        Ok(segments)
    }
}

fn not_found(subject: &str) -> ResourceLibraryError {
    ResourceLibraryError::new(
        ResourceLibraryErrorKind::NotFound,
        format!("library {subject} does not exist"),
    )
}
