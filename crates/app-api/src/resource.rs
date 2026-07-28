use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ResourceRefDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImageResourceKindDto {
    SourceImage,
    ReferenceImage,
    ControlNetImage,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImportImageResourceRequestDto {
    pub kind: ImageResourceKindDto,
    pub image_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImportImageResourceResponseDto {
    pub resource: ResourceRefDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ReleaseImportedImageResourcesRequestDto {
    pub resources: Vec<ResourceRefDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ReleaseImportedImageResourcesResponseDto {
    pub released: usize,
    pub resources_deleted: usize,
    pub blobs_deleted: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GetResourceImageRequestDto {
    pub resource: ResourceRefDto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImageExportFormatDto {
    PngOriginal,
    PngSanitized,
    Jpeg,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SaveResourceImageRequestDto {
    pub resource: ResourceRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ImageExportFormatDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_file_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CopyResourceImageRequestDto {
    pub resource: ResourceRefDto,
    pub format: ImageExportFormatDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SaveResourceImagesZipEntryDto {
    pub resource: ResourceRefDto,
    pub file_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SaveResourceImagesZipRequestDto {
    pub entries: Vec<SaveResourceImagesZipEntryDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_file_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ResourceImageDto {
    pub image_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}
