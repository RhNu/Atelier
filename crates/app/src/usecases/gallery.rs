use super::{
    AppError, AppResult, GalleryItemId, GalleryPageDto, GalleryQueryDto, GallerySafetyOverrideDto,
    WorkspaceSession, gallery_image_reference_to_dto, gallery_item_to_dto, gallery_page_to_dto,
    gallery_query_to_domain, image_reference_target_to_domain, safety_override_to_domain,
};
use atelier_adapter_database::{GalleryHardDeletePlan, GalleryTransientOwner};
use atelier_app_api::gallery::{
    DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryItemDetailDto,
    GalleryItemDetailRequestDto, RescanGallerySafetyRequestDto, RescanGallerySafetyResponseDto,
};
use atelier_artifacts::{ArtifactId, ArtifactSource};
use atelier_gallery::{GalleryItem, GallerySafetyState};
use atelier_safety::{SafetyScanInput, SafetyScanner};

pub struct GalleryUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
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

    pub async fn detail(
        &self,
        request: GalleryItemDetailRequestDto,
    ) -> AppResult<GalleryItemDetailDto> {
        let item_id = GalleryItemId::new(&request.item_id);
        let item = self
            .app
            .inner
            .gallery
            .get_items(&[item_id])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::new("not_found", "gallery item does not exist"))?;
        let artifact = self
            .app
            .inner
            .artifacts
            .get_artifact(&ArtifactId::new(item.artifact_id.as_str()))
            .await
            .map_err(|error| AppError::new("artifact", error.to_string()))?;
        Ok(GalleryItemDetailDto {
            item_id: request.item_id,
            embedded_metadata_json: artifact
                .and_then(|artifact| artifact.metadata.embedded_metadata_json),
        })
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

    pub async fn rescan_safety(
        &self,
        request: RescanGallerySafetyRequestDto,
    ) -> AppResult<RescanGallerySafetyResponseDto> {
        let _rescan_guard = self.app.inner.gallery_safety_rescan.lock().await;
        let item_ids = if request.item_ids.is_empty() {
            self.app
                .inner
                .gallery_index
                .pending_safety_item_ids(1_000)
                .map_err(AppError::from)?
        } else {
            request
                .item_ids
                .into_iter()
                .map(GalleryItemId::new)
                .collect()
        };
        let items = self.app.inner.gallery.get_items(&item_ids).await?;
        let scanner = self.app.inner.safety_scanner.clone();
        let reader = self.app.inner.resource_reader.clone();
        let now_ms = super::unix_timestamp_ms();
        let Some(scanner) = scanner else {
            for item in &items {
                self.app
                    .inner
                    .gallery
                    .set_safety_state(
                        &item.id,
                        GallerySafetyState::Unavailable {
                            message: "automatic safety scanning is unavailable".to_owned(),
                        },
                    )
                    .await?;
            }
            return Ok(RescanGallerySafetyResponseDto {
                requested: items.len(),
                scanned: 0,
                failed: 0,
                unavailable: items.len(),
            });
        };

        let mut response = RescanGallerySafetyResponseDto {
            requested: items.len(),
            scanned: 0,
            failed: 0,
            unavailable: 0,
        };
        for item in items {
            let state = scan_gallery_item(scanner.as_ref(), &reader, &item, now_ms).await;
            match &state {
                GallerySafetyState::Scanned(_) => response.scanned += 1,
                GallerySafetyState::Failed { .. } => response.failed += 1,
                GallerySafetyState::Unavailable { .. } => response.unavailable += 1,
                GallerySafetyState::Unscanned => {}
            }
            self.app
                .inner
                .gallery
                .set_safety_state(&item.id, state)
                .await?;
        }
        Ok(response)
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
            let cleanup = self.app.inner.resources.cleanup_delete_pending().await?;
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
        let deleted = self
            .app
            .inner
            .gallery_index
            .hard_delete(&plans)
            .await
            .map_err(AppError::from)?;

        let cleanup = self.app.inner.resources.cleanup_delete_pending().await?;
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

async fn scan_gallery_item(
    scanner: &dyn SafetyScanner,
    reader: &crate::ports::AppResourceReader,
    item: &GalleryItem,
    attempted_at_ms: u64,
) -> GallerySafetyState {
    let content = match reader.read_resource_bytes(&item.primary_resource).await {
        Ok(content) => content,
        Err(error) => {
            return GallerySafetyState::Failed {
                message: error.to_string(),
                attempted_at_ms,
            };
        }
    };
    match scanner
        .scan_image(SafetyScanInput {
            resource: item.primary_resource.clone(),
            bytes: content.bytes,
            mime_type: None,
        })
        .await
    {
        Ok(assessment) => {
            GallerySafetyState::Scanned(Box::new(assessment.with_assessed_at_ms(attempted_at_ms)))
        }
        Err(error) => GallerySafetyState::Failed {
            message: error.to_string(),
            attempted_at_ms,
        },
    }
}

fn transient_resource_owner(item: &GalleryItem) -> GalleryTransientOwner {
    match &item.source {
        ArtifactSource::GenerationJob { job_id, .. } => GalleryTransientOwner {
            kind: "job",
            local_id: job_id.clone(),
        },
        ArtifactSource::DirectorRun { run_id } => GalleryTransientOwner {
            kind: "director_run",
            local_id: run_id.clone(),
        },
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
