use atelier_app_api::gallery::{
    GalleryImageReferenceDto, GalleryImageReferenceRequestDto, GalleryItemDto, GalleryPageDto,
    GalleryQueryDto, SetGallerySafetyOverrideRequestDto,
};

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
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
        Self::command_result(self.current_app()?.gallery().query(request).await)
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
            self.current_app()?
                .gallery()
                .set_safety_override(&request.item_id, request.manual_safety_override)
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
        Self::command_result(self.current_app()?.gallery().image_reference(request).await)
    }
}
