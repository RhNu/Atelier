use super::{ResourceId, ResourceRef, ResourceRefDto, VariantId};

pub fn resource_ref_to_dto(value: &ResourceRef) -> ResourceRefDto {
    ResourceRefDto {
        id: value.id.as_str().to_owned(),
        variant_id: value.variant_id.as_ref().map(|id| id.as_str().to_owned()),
    }
}

pub fn resource_ref_from_dto(value: ResourceRefDto) -> ResourceRef {
    ResourceRef::new(
        ResourceId::new(value.id),
        value.variant_id.map(VariantId::new),
    )
}
