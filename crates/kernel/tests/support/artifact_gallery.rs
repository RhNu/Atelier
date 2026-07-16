use async_trait::async_trait;
use atelier_artifacts::{
    ArtifactRecord, ArtifactRepository, ArtifactResourceReader, ArtifactResult,
};
use atelier_gallery::{GalleryIndex, GalleryItem, GalleryItemId, GalleryResult};
use atelier_resource_catalog::{ResourceMetadata, ResourceRecord, ResourceRef, ResourceState};

use super::MemoryKernelPorts;

#[async_trait]
impl ArtifactRepository for MemoryKernelPorts {
    async fn insert_artifact(&self, record: ArtifactRecord) -> ArtifactResult<()> {
        self.state
            .lock()
            .unwrap()
            .artifacts
            .insert(record.id.as_str().to_owned(), record);
        Ok(())
    }

    async fn delete_artifacts(
        &self,
        ids: &[atelier_artifacts::ArtifactId],
    ) -> ArtifactResult<usize> {
        let mut state = self.state.lock().unwrap();
        let mut deleted = 0;
        for id in ids {
            if state.artifacts.remove(id.as_str()).is_some() {
                deleted += 1;
            }
        }
        Ok(deleted)
    }
}

#[async_trait]
impl ArtifactResourceReader for MemoryKernelPorts {
    async fn get_artifact_resource(
        &self,
        reference: &ResourceRef,
    ) -> ArtifactResult<ResourceRecord> {
        Ok(ResourceRecord {
            id: reference.id.clone(),
            kind: self.state.lock().unwrap().resources[reference.id.as_str()].kind,
            lifecycle: atelier_resource_catalog::ResourceLifecycle::JobScoped,
            state: ResourceState::Ready,
            blob_id: atelier_resource_catalog::BlobId::new("blob"),
            metadata: ResourceMetadata::default(),
        })
    }
}

#[async_trait]
impl GalleryIndex for MemoryKernelPorts {
    async fn upsert_item(&self, item: GalleryItem) -> GalleryResult<()> {
        self.state
            .lock()
            .unwrap()
            .gallery_items
            .insert(item.id.as_str().to_owned(), item);
        Ok(())
    }

    async fn get_item(&self, id: &GalleryItemId) -> GalleryResult<Option<GalleryItem>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .gallery_items
            .get(id.as_str())
            .cloned())
    }

    async fn query_items(
        &self,
        query: atelier_gallery::GalleryQuery,
    ) -> GalleryResult<Vec<GalleryItem>> {
        Ok(query.apply(self.state.lock().unwrap().gallery_items.values().cloned()))
    }

    async fn set_safety_override(
        &self,
        id: &GalleryItemId,
        manual_safety_override: Option<atelier_gallery::GallerySafetyOverride>,
    ) -> GalleryResult<GalleryItem> {
        let mut state = self.state.lock().unwrap();
        let item = state
            .gallery_items
            .get_mut(id.as_str())
            .ok_or_else(|| atelier_gallery::GalleryError::not_found("missing item"))?;
        item.manual_safety_override = manual_safety_override;
        Ok(item.clone())
    }

    async fn delete_items(&self, ids: &[GalleryItemId]) -> GalleryResult<Vec<GalleryItem>> {
        let mut state = self.state.lock().unwrap();
        let mut deleted = Vec::new();
        for id in ids {
            if let Some(item) = state.gallery_items.remove(id.as_str()) {
                deleted.push(item);
            }
        }
        Ok(deleted)
    }
}
