use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRefDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImageInputDto {
    ResourceRef { resource: ResourceRefDto },
    InlineBase64 { image_base64: String },
}

impl ImageInputDto {
    #[must_use]
    pub fn resource(id: impl Into<String>) -> Self {
        Self::ResourceRef {
            resource: ResourceRefDto {
                id: id.into(),
                variant_id: None,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageResourceKindDto {
    SourceImage,
    ReferenceImage,
    ControlNetImage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportImageResourceRequestDto {
    pub kind: ImageResourceKindDto,
    pub image_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportImageResourceResponseDto {
    pub resource: ResourceRefDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GetResourceImageRequestDto {
    pub resource: ResourceRefDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceImageDto {
    pub image_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
