use crate::mapping::resource_ref_from_dto;
use crate::ports::AppResourceReader;
use crate::{AppError, AppResult};
use atelier_app_api::generation::CharacterReferenceDto;
use atelier_app_api::resource::ImageInputDto;
use atelier_generation::CharacterReference;
use atelier_precise_reference::{PreciseReferenceImage, prepare_reference};
use atelier_resource_catalog::ResourceKind;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub struct ImageInputResolver<'a> {
    reader: &'a AppResourceReader,
}

impl<'a> ImageInputResolver<'a> {
    pub const fn new(reader: &'a AppResourceReader) -> Self {
        Self { reader }
    }

    pub async fn resolve(&self, input: ImageInputDto) -> AppResult<String> {
        self.read(input).await.map(|image| image.payload)
    }

    pub async fn reference(&self, input: CharacterReferenceDto) -> AppResult<CharacterReference> {
        let image = self.read(input.image).await?;
        prepare_reference(
            image,
            crate::mapping::character_reference_type_to_domain(input.reference_type),
            input.fidelity,
            input.strength,
        )
        .map_err(|error| AppError::new("invalid_reference", error.to_string()))
    }

    async fn read(&self, input: ImageInputDto) -> AppResult<PreciseReferenceImage> {
        match input {
            ImageInputDto::InlineBase64 { image_base64 } => Ok(PreciseReferenceImage {
                kind: ResourceKind::ReferenceImage,
                payload: image_base64,
            }),
            ImageInputDto::ResourceRef { resource } => {
                let content = self
                    .reader
                    .read_resource_bytes(&resource_ref_from_dto(resource))
                    .await?;
                Ok(PreciseReferenceImage {
                    kind: content.kind,
                    payload: STANDARD.encode(content.bytes),
                })
            }
        }
    }
}
