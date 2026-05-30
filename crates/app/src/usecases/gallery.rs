use super::{
    AppError, AppResult, AtelierApp, GalleryItemId, GalleryPageDto, GalleryQueryDto,
    GallerySafetyOverrideDto, gallery_image_reference_to_dto, gallery_item_to_dto,
    gallery_page_to_dto, gallery_query_to_domain, image_reference_target_to_domain,
    safety_override_to_domain,
};
use atelier_app_api::gallery::{DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto};
use atelier_artifacts::{ArtifactId, ArtifactSource};
use atelier_gallery::GalleryItem;
use atelier_jobs::RunHistoryRepository;
use atelier_resource_catalog::{
    ResourceCatalogErrorKind, ResourceId, ResourceOwner, ResourceOwnerKind, ResourceRelation,
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

        let deleted_item_ids = existing_items
            .iter()
            .map(|item| item.id.as_str().to_owned())
            .collect::<Vec<_>>();
        let artifact_ids = existing_items
            .iter()
            .map(|item| item.artifact_id.clone())
            .collect::<Vec<_>>();
        self.delete_artifacts_and_release_resources(&existing_items, &artifact_ids)
            .await?;
        self.app
            .inner
            .run_history
            .delete_run_outputs_by_item_ids(&deleted_item_ids)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        let deleted_items = self.app.inner.gallery.delete_items(&item_ids).await?;

        let cleanup = {
            let kernel = self.app.inner.kernel.lock().await;
            let cleanup = kernel.ports().resources.cleanup_delete_pending().await?;
            drop(kernel);
            cleanup
        };
        Ok(DeleteGalleryItemsResponseDto {
            deleted: deleted_items.len(),
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

    async fn delete_artifacts_and_release_resources(
        &self,
        deleted_items: &[GalleryItem],
        artifact_ids: &[ArtifactId],
    ) -> AppResult<()> {
        let kernel = self.app.inner.kernel.lock().await;
        kernel
            .ports()
            .artifacts
            .delete_artifacts(artifact_ids)
            .await
            .map_err(|error| AppError::new("artifact", error.to_string()))?;
        for item in deleted_items {
            release_gallery_item_resources(&kernel.ports().resources, item).await?;
        }
        drop(kernel);
        Ok(())
    }
}

async fn release_gallery_item_resources(
    resources: &crate::ports::AppResourceCatalog,
    item: &GalleryItem,
) -> AppResult<()> {
    let gallery_owner = ResourceOwner::new(ResourceOwnerKind::GalleryItem, item.id.as_str());
    let transient_owner = transient_resource_owner(item);
    let force_delete_pending = matches!(item.source, ArtifactSource::DirectorRun { .. });
    for resource_id in gallery_resource_ids(item) {
        detach_owner_if_ready(resources, &resource_id, &gallery_owner).await?;
        if let Some(owner) = &transient_owner {
            detach_owner_if_ready(resources, &resource_id, owner).await?;
        }
        if force_delete_pending {
            mark_delete_pending_if_unowned(resources, &resource_id).await?;
        }
    }
    Ok(())
}

async fn detach_owner_if_ready(
    resources: &crate::ports::AppResourceCatalog,
    resource_id: &ResourceId,
    owner: &ResourceOwner,
) -> AppResult<()> {
    match resources
        .detach_owner(resource_id, owner, ResourceRelation::Primary)
        .await
    {
        Ok(_outcome) => Ok(()),
        Err(error) if error.kind == ResourceCatalogErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    }
}

async fn mark_delete_pending_if_unowned(
    resources: &crate::ports::AppResourceCatalog,
    resource_id: &ResourceId,
) -> AppResult<()> {
    match resources.mark_delete_pending_if_unowned(resource_id).await {
        Ok(_marked) => Ok(()),
        Err(error) if error.kind == ResourceCatalogErrorKind::NotFound => Ok(()),
        Err(error) => Err(AppError::from(error)),
    }
}

fn transient_resource_owner(item: &GalleryItem) -> Option<ResourceOwner> {
    match &item.source {
        ArtifactSource::GenerationJob { job_id, .. } => {
            Some(ResourceOwner::new(ResourceOwnerKind::Job, job_id.clone()))
        }
        ArtifactSource::DirectorRun { run_id } => Some(ResourceOwner::new(
            ResourceOwnerKind::DirectorRun,
            run_id.clone(),
        )),
        ArtifactSource::Import { .. } => None,
    }
}

fn gallery_resource_ids(item: &GalleryItem) -> Vec<ResourceId> {
    let mut ids = vec![item.primary_resource.id.clone()];
    for asset in &item.assets {
        if !ids.contains(&asset.resource.id) {
            ids.push(asset.resource.id.clone());
        }
    }
    ids
}
