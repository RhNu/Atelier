use super::{
    AppError, AppResult, AtelierApp, GalleryItemId, GalleryPageDto, GalleryQueryDto,
    GallerySafetyOverrideDto, gallery_image_reference_to_dto, gallery_item_to_dto,
    gallery_page_to_dto, gallery_query_to_domain, image_reference_target_to_domain,
    safety_override_to_domain,
};

pub struct GalleryUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> GalleryUseCases<'_, S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    pub async fn query(&self, query: GalleryQueryDto) -> AppResult<GalleryPageDto> {
        let offset = query.offset;
        let limit = query.limit;
        let page_query = gallery_query_to_domain(&query)?;
        let total_query = gallery_query_to_domain(&GalleryQueryDto {
            offset: 0,
            limit: usize::try_from(i64::MAX).unwrap_or(usize::MAX),
            ..query
        })?;
        let items = self
            .app
            .inner
            .gallery
            .query(page_query)
            .await
            .map_err(AppError::from)?;
        let total = self
            .app
            .inner
            .gallery
            .query(total_query)
            .await
            .map_err(AppError::from)?
            .len();
        Ok(gallery_page_to_dto(items, offset, limit, total))
    }

    pub async fn set_safety_override(
        &self,
        item_id: &str,
        override_value: Option<GallerySafetyOverrideDto>,
    ) -> AppResult<nai_atelier_app_api::gallery::GalleryItemDto> {
        self.app
            .inner
            .gallery
            .set_safety_override(
                &GalleryItemId::new(item_id),
                override_value.map(safety_override_to_domain),
            )
            .await
            .map(gallery_item_to_dto)
            .map_err(AppError::from)
    }

    pub async fn image_reference(
        &self,
        request: nai_atelier_app_api::gallery::GalleryImageReferenceRequestDto,
    ) -> AppResult<nai_atelier_app_api::gallery::GalleryImageReferenceDto> {
        self.app
            .inner
            .gallery
            .image_reference_for(
                &GalleryItemId::new(request.item_id),
                image_reference_target_to_domain(request.target),
            )
            .await
            .map(gallery_image_reference_to_dto)
            .map_err(AppError::from)
    }
}
