use atelier_adapter_image_codec::ImageCodec;
use atelier_app_api::resource::{
    GetResourceImageRequestDto, ImageResourceKindDto, ImportImageResourceRequestDto,
    ImportImageResourceResponseDto, ReleaseImportedImageResourcesRequestDto,
    ReleaseImportedImageResourcesResponseDto, ResourceImageDto,
};
use atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceCleanupReport, ResourceId, ResourceKind,
    ResourceLifecycle, ResourceOwner, ResourceOwnerKind, ResourceRelation,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppResult;
use crate::app::WorkspaceSession;
use crate::mapping::{resource_ref_from_dto, resource_ref_to_dto};

pub struct ResourceUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}

impl<S, F, E> ResourceUseCases<'_, S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    pub async fn import_image(
        &self,
        request: ImportImageResourceRequestDto,
    ) -> AppResult<ImportImageResourceResponseDto> {
        let bytes = STANDARD.decode(request.image_base64.trim())?;
        let kind = image_resource_kind_to_domain(request.kind);
        let resource_id = ResourceId::new(format!(
            "resource:import:{}:{}",
            resource_kind_slug(kind),
            unix_timestamp_nanos()
        ));
        let resource = self
            .app
            .inner
            .resources
            .register_resource(RegisterResourceRequest {
                resource_id,
                kind,
                lifecycle: ResourceLifecycle::Cache,
                owner: import_staging_owner(),
                relation: image_resource_relation(request.kind),
                blob: BlobWriteIntent::Bytes(bytes),
            })
            .await?;
        Ok(ImportImageResourceResponseDto {
            resource: resource_ref_to_dto(&resource),
        })
    }

    pub async fn release_imported_images(
        &self,
        request: ReleaseImportedImageResourcesRequestDto,
    ) -> AppResult<ReleaseImportedImageResourcesResponseDto> {
        let mut released = 0;
        let mut resource_ids = std::collections::BTreeSet::new();
        for resource in request.resources {
            if resource.variant_id.is_none() && resource.id.starts_with("resource:import:") {
                resource_ids.insert(resource.id);
            }
        }
        let catalog = &self.app.inner.resources;
        let owner = import_staging_owner();
        let links = catalog.list_links_by_owner(&owner).await?;
        for link in links
            .iter()
            .filter(|link| resource_ids.contains(link.resource_id.as_str()))
        {
            catalog
                .detach_owner(&link.resource_id, &owner, link.relation)
                .await?;
            released += 1;
        }
        let cleanup = if released == 0 {
            ResourceCleanupReport::default()
        } else {
            catalog.cleanup_delete_pending().await?
        };
        Ok(ReleaseImportedImageResourcesResponseDto {
            released,
            resources_deleted: cleanup.resources_deleted,
            blobs_deleted: cleanup.blobs_deleted,
        })
    }

    pub async fn release_all_imported_images(
        &self,
    ) -> AppResult<ReleaseImportedImageResourcesResponseDto> {
        let catalog = &self.app.inner.resources;
        let owner = import_staging_owner();
        let links = catalog.list_links_by_owner(&owner).await?;
        for link in &links {
            catalog
                .detach_owner(&link.resource_id, &owner, link.relation)
                .await?;
        }
        let cleanup = if links.is_empty() {
            ResourceCleanupReport::default()
        } else {
            catalog.cleanup_delete_pending().await?
        };
        Ok(ReleaseImportedImageResourcesResponseDto {
            released: links.len(),
            resources_deleted: cleanup.resources_deleted,
            blobs_deleted: cleanup.blobs_deleted,
        })
    }

    pub async fn get_image(
        &self,
        request: GetResourceImageRequestDto,
    ) -> AppResult<ResourceImageDto> {
        let reference = resource_ref_from_dto(request.resource);
        let content = self
            .app
            .inner
            .resource_reader
            .read_resource_bytes(&reference)
            .await?;
        let mime_type = ImageCodec::probe(&content.bytes)
            .ok()
            .map(|info| info.mime_type);
        Ok(ResourceImageDto {
            image_base64: STANDARD.encode(content.bytes),
            mime_type,
        })
    }
}

const fn image_resource_kind_to_domain(value: ImageResourceKindDto) -> ResourceKind {
    match value {
        ImageResourceKindDto::SourceImage => ResourceKind::SourceImage,
        ImageResourceKindDto::ReferenceImage => ResourceKind::ReferenceImage,
        ImageResourceKindDto::ControlNetImage => ResourceKind::ControlNetImage,
    }
}

const fn image_resource_relation(value: ImageResourceKindDto) -> ResourceRelation {
    match value {
        ImageResourceKindDto::SourceImage => ResourceRelation::Source,
        ImageResourceKindDto::ReferenceImage | ImageResourceKindDto::ControlNetImage => {
            ResourceRelation::Reference
        }
    }
}

const fn resource_kind_slug(value: ResourceKind) -> &'static str {
    match value {
        ResourceKind::SourceImage => "source",
        ResourceKind::ReferenceImage => "reference",
        ResourceKind::ControlNetImage => "controlnet",
        _ => "image",
    }
}

fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

const IMPORT_STAGING_OWNER_ID: &str = "user-image-inputs";

fn import_staging_owner() -> ResourceOwner {
    ResourceOwner::new(ResourceOwnerKind::ImportStaging, IMPORT_STAGING_OWNER_ID)
}
