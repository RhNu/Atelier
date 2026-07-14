use atelier_artifacts::ArtifactRecord;
use atelier_safety::SafetyAssessment;

use crate::{
    GalleryError, GalleryImageReference, GalleryIndex, GalleryItem, GalleryItemId, GalleryQuery,
    GalleryResult, GallerySafetyOverride, ImageReferenceTarget,
};

#[derive(Clone, Debug)]
pub struct GalleryService<I> {
    index: I,
}

impl<I> GalleryService<I> {
    #[must_use]
    pub const fn new(index: I) -> Self {
        Self { index }
    }
}

impl<I> GalleryService<I>
where
    I: GalleryIndex,
{
    /// Indexes an artifact-backed gallery item.
    ///
    /// # Errors
    /// Returns an error when the gallery index cannot persist the item.
    pub async fn index_artifact(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety_assessment: Option<SafetyAssessment>,
    ) -> GalleryResult<GalleryItem> {
        let item_id = GalleryItemId::from_artifact_id(&artifact.id);
        let manual_safety_override = self
            .index
            .get_item(&item_id)
            .await?
            .and_then(|item| item.manual_safety_override);
        let mut item = GalleryItem::from_artifact(artifact, indexed_at_ms, safety_assessment);
        item.manual_safety_override = manual_safety_override;
        self.index.upsert_item(item.clone()).await?;
        Ok(item)
    }

    /// Queries indexed gallery items.
    ///
    /// # Errors
    /// Returns an error when the gallery index cannot be queried.
    pub async fn query(&self, query: GalleryQuery) -> GalleryResult<Vec<GalleryItem>> {
        self.index.query_items(query).await
    }

    /// Counts gallery items matching a query without materializing the page.
    ///
    /// # Errors
    /// Returns an error when the gallery index cannot be queried.
    pub async fn count(&self, query: GalleryQuery) -> GalleryResult<usize> {
        self.index.count_items(query).await
    }

    /// Returns existing gallery items for the given IDs, ignoring missing IDs.
    ///
    /// # Errors
    /// Returns an error when the gallery index cannot be queried.
    pub async fn get_items(&self, item_ids: &[GalleryItemId]) -> GalleryResult<Vec<GalleryItem>> {
        let mut items = Vec::new();
        for item_id in item_ids {
            if let Some(item) = self.index.get_item(item_id).await? {
                items.push(item);
            }
        }
        Ok(items)
    }

    /// Deletes indexed gallery items and returns the records that existed.
    ///
    /// # Errors
    /// Returns an error when the gallery index cannot be updated.
    pub async fn delete_items(
        &self,
        item_ids: &[GalleryItemId],
    ) -> GalleryResult<Vec<GalleryItem>> {
        self.index.delete_items(item_ids).await
    }

    /// Sets or clears a manual safety override.
    ///
    /// # Errors
    /// Returns an error when the gallery item does not exist or persistence fails.
    pub async fn set_safety_override(
        &self,
        item_id: &GalleryItemId,
        manual_safety_override: Option<GallerySafetyOverride>,
    ) -> GalleryResult<GalleryItem> {
        self.index
            .set_safety_override(item_id, manual_safety_override)
            .await
    }

    /// Returns a pure image reference for downstream feature handoff.
    ///
    /// # Errors
    /// Returns an error when the gallery item does not exist or cannot be read.
    pub async fn image_reference_for(
        &self,
        item_id: &GalleryItemId,
        target: ImageReferenceTarget,
    ) -> GalleryResult<GalleryImageReference> {
        let item = self
            .index
            .get_item(item_id)
            .await?
            .ok_or_else(|| GalleryError::not_found("gallery item does not exist"))?;
        Ok(item.image_reference(target))
    }
}
