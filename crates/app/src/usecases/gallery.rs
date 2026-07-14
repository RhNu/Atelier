use super::{
    AppError, AppResult, AtelierApp, GalleryItemId, GalleryPageDto, GalleryQueryDto,
    GallerySafetyOverrideDto, gallery_image_reference_to_dto, gallery_item_to_dto,
    gallery_page_to_dto, gallery_query_to_domain, image_reference_target_to_domain,
    safety_override_to_domain,
};
use atelier_adapter_database::{GalleryHardDeletePlan, GalleryTransientOwner};
use atelier_app_api::gallery::{DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto};
use atelier_artifacts::ArtifactSource;
use atelier_gallery::GalleryItem;

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
        let total_query = gallery_query_to_domain(&query)?;
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
            .count(total_query)
            .await
            .map_err(AppError::from)?;
        Ok(gallery_page_to_dto(items, offset, limit, total))
    }

    pub async fn set_safety_override(
        &self,
        item_id: &str,
        override_value: Option<GallerySafetyOverrideDto>,
    ) -> AppResult<atelier_app_api::gallery::GalleryItemDto> {
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

    pub async fn delete_items(
        &self,
        request: DeleteGalleryItemsRequestDto,
    ) -> AppResult<DeleteGalleryItemsResponseDto> {
        let item_ids = request
            .item_ids
            .into_iter()
            .map(GalleryItemId::new)
            .collect::<Vec<_>>();
        let existing_items = self.app.inner.gallery.get_items(&item_ids).await?;
        if existing_items.is_empty() {
            let cleanup = {
                let kernel = self.app.inner.kernel.lock().await;
                let cleanup = kernel.ports().resources.cleanup_delete_pending().await?;
                drop(kernel);
                cleanup
            };
            return Ok(DeleteGalleryItemsResponseDto {
                deleted: 0,
                resources_released: cleanup.resources_deleted,
                blobs_deleted: cleanup.blobs_deleted,
            });
        }

        let plans = existing_items
            .iter()
            .map(hard_delete_plan)
            .collect::<Vec<_>>();
        let deleted = {
            let kernel = self.app.inner.kernel.lock().await;
            let deleted = self
                .app
                .inner
                .gallery_index
                .hard_delete(&plans)
                .await
                .map_err(AppError::from)?;
            drop(kernel);
            deleted
        };

        let cleanup = {
            let kernel = self.app.inner.kernel.lock().await;
            let cleanup = kernel.ports().resources.cleanup_delete_pending().await?;
            drop(kernel);
            cleanup
        };
        Ok(DeleteGalleryItemsResponseDto {
            deleted,
            resources_released: cleanup.resources_deleted,
            blobs_deleted: cleanup.blobs_deleted,
        })
    }

    pub async fn image_reference(
        &self,
        request: atelier_app_api::gallery::GalleryImageReferenceRequestDto,
    ) -> AppResult<atelier_app_api::gallery::GalleryImageReferenceDto> {
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

fn transient_resource_owner(item: &GalleryItem) -> Option<GalleryTransientOwner> {
    match &item.source {
        ArtifactSource::GenerationJob { job_id, .. } => Some(GalleryTransientOwner {
            kind: "job",
            local_id: job_id.clone(),
        }),
        ArtifactSource::DirectorRun { run_id } => Some(GalleryTransientOwner {
            kind: "director_run",
            local_id: run_id.clone(),
        }),
        ArtifactSource::Import { .. } => None,
    }
}

fn hard_delete_plan(item: &GalleryItem) -> GalleryHardDeletePlan {
    let mut resource_ids = vec![item.primary_resource.id.as_str().to_owned()];
    for asset in &item.assets {
        let id = asset.resource.id.as_str().to_owned();
        if !resource_ids.contains(&id) {
            resource_ids.push(id);
        }
    }
    GalleryHardDeletePlan {
        item_id: item.id.as_str().to_owned(),
        artifact_id: item.artifact_id.as_str().to_owned(),
        resource_ids,
        transient_owner: transient_resource_owner(item),
        force_delete_pending: matches!(item.source, ArtifactSource::DirectorRun { .. }),
    }
}
