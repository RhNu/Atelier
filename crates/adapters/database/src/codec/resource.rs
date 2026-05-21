use super::{Deserialize, ResourceId, ResourceRef, Serialize, VariantId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceRefDto {
    id: String,
    variant_id: Option<String>,
}

impl From<&ResourceRef> for ResourceRefDto {
    fn from(value: &ResourceRef) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            variant_id: value.variant_id.as_ref().map(|id| id.as_str().to_owned()),
        }
    }
}

impl ResourceRefDto {
    pub fn into_domain(self) -> ResourceRef {
        ResourceRef::new(
            ResourceId::new(self.id),
            self.variant_id.map(VariantId::new),
        )
    }
}
