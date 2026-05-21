use atelier_adapter_image_codec::ImageCodec;
use atelier_app_api::resource::{
    GetResourceImageRequestDto, ImageResourceKindDto, ImportImageResourceRequestDto,
    ImportImageResourceResponseDto, ResourceImageDto,
};
use atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceId, ResourceKind, ResourceLifecycle,
    ResourceOwner, ResourceOwnerKind, ResourceRelation,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppResult;
use crate::app::AtelierApp;
use crate::mapping::{resource_ref_from_dto, resource_ref_to_dto};

pub struct ResourceUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
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
        let kernel = self.app.inner.kernel.lock().await;
        let resource = kernel
            .ports()
            .resources
            .register_resource(RegisterResourceRequest {
                resource_id,
                kind,
                lifecycle: ResourceLifecycle::WorkspaceScoped,
                owner: ResourceOwner::new(ResourceOwnerKind::Workspace, "workspace"),
                relation: image_resource_relation(request.kind),
                blob: BlobWriteIntent::Bytes(bytes),
            })
            .await?;
        drop(kernel);
        Ok(ImportImageResourceResponseDto {
            resource: resource_ref_to_dto(&resource),
        })
    }

    pub async fn get_image(
        &self,
        request: GetResourceImageRequestDto,
    ) -> AppResult<ResourceImageDto> {
        let reference = resource_ref_from_dto(request.resource);
        let content = {
            let kernel = self.app.inner.kernel.lock().await;
            kernel
                .ports()
                .resource_reader
                .read_resource_bytes(&reference)
                .await?
        };
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
