use async_trait::async_trait;

use crate::{
    GalleryItem, GalleryItemId, GalleryQuery, GalleryResult, GallerySafetyOverride,
    GallerySafetyState,
};

#[async_trait]
pub trait GalleryIndex: Send + Sync {
    async fn upsert_item(&self, item: GalleryItem) -> GalleryResult<()>;

    async fn get_item(&self, id: &GalleryItemId) -> GalleryResult<Option<GalleryItem>>;

    async fn query_items(&self, query: GalleryQuery) -> GalleryResult<Vec<GalleryItem>>;

    async fn count_items(&self, mut query: GalleryQuery) -> GalleryResult<usize> {
        query.offset = 0;
        query.limit = usize::MAX;
        self.query_items(query).await.map(|items| items.len())
    }

    async fn delete_items(&self, ids: &[GalleryItemId]) -> GalleryResult<Vec<GalleryItem>>;

    async fn set_safety_override(
        &self,
        id: &GalleryItemId,
        manual_safety_override: Option<GallerySafetyOverride>,
    ) -> GalleryResult<GalleryItem>;

    async fn set_safety_state(
        &self,
        id: &GalleryItemId,
        safety: GallerySafetyState,
    ) -> GalleryResult<GalleryItem>;
}
