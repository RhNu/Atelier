use atelier_app_api::gallery::{
    DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryImageReferenceDto,
    GalleryImageReferenceRequestDto, GalleryItemDetailDto, GalleryItemDetailRequestDto,
    GalleryItemDto, GalleryPageDto, GalleryQueryDto, RescanGallerySafetyRequestDto,
    RescanGallerySafetyResponseDto, SetGallerySafetyOverrideRequestDto,
};

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Queries indexed gallery items.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or gallery storage fails.
    pub async fn query_gallery(&self, request: GalleryQueryDto) -> CommandResult<GalleryPageDto> {
        Self::command_result(self.current_session()?.gallery().query(request).await)
    }

    /// Loads heavyweight metadata for one selected gallery item.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the item cannot be read.
    pub async fn get_gallery_item_detail(
        &self,
        request: GalleryItemDetailRequestDto,
    ) -> CommandResult<GalleryItemDetailDto> {
        Self::command_result(self.current_session()?.gallery().detail(request).await)
    }

    /// Sets or clears a manual gallery safety override.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the item is missing, or gallery storage fails.
    pub async fn set_gallery_safety_override(
        &self,
        request: SetGallerySafetyOverrideRequestDto,
    ) -> CommandResult<GalleryItemDto> {
        Self::command_result(
            self.current_session()?
                .gallery()
                .set_safety_override(&request.item_id, request.manual_safety_override)
                .await,
        )
    }

    /// Rescans selected gallery items, or every pending item when no IDs are supplied.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or gallery safety persistence fails.
    pub async fn rescan_gallery_safety(
        &self,
        request: RescanGallerySafetyRequestDto,
    ) -> CommandResult<RescanGallerySafetyResponseDto> {
        Self::command_result(
            self.current_session()?
                .gallery()
                .rescan_safety(request)
                .await,
        )
    }

    /// Deletes gallery items, associated artifact/output index rows, and released resources.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or gallery/resource storage fails.
    pub async fn delete_gallery_items(
        &self,
        request: DeleteGalleryItemsRequestDto,
    ) -> CommandResult<DeleteGalleryItemsResponseDto> {
        Self::command_result(
            self.current_session()?
                .gallery()
                .delete_items(request)
                .await,
        )
    }

    /// Returns a resource reference suitable for downstream image handoff.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the item is missing, or gallery storage fails.
    pub async fn gallery_image_reference(
        &self,
        request: GalleryImageReferenceRequestDto,
    ) -> CommandResult<GalleryImageReferenceDto> {
        Self::command_result(
            self.current_session()?
                .gallery()
                .image_reference(request)
                .await,
        )
    }
}
