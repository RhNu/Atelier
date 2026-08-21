use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::generation::ImageModelDto;
use crate::resource::ResourceRefDto;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum VibeExportFormatDto {
    #[default]
    Naiv4vibe,
    Naiv4vibebundle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImportVibeDocumentRequestDto {
    pub file_name: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImportEmbeddedPngVibeDocumentRequestDto {
    pub file_name: String,
    pub png_bytes_base64: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExportVibeDocumentRequestDto {
    pub vibe_ids: Vec<String>,
    pub format: VibeExportFormatDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct EnsureVibeEncodingRequestDto {
    pub vibe_id: String,
    pub source_sha256: String,
    pub image: String,
    pub model: ImageModelDto,
    pub information_extracted: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct VibeEncodingConfigDto {
    pub model: ImageModelDto,
    pub information_extracted: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ListVibeDocumentsRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ImageModelDto>,
    pub offset: usize,
    pub limit: usize,
    pub include_hidden: bool,
}

impl Default for ListVibeDocumentsRequestDto {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            include_hidden: false,
            model: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RenameVibeDocumentRequestDto {
    pub vibe_id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SetVibeDocumentHiddenRequestDto {
    pub vibe_id: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GetVibeDocumentRequestDto {
    pub vibe_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct VibeDocumentEntryDto {
    pub vibe_id: String,
    pub display_name: String,
    pub has_image: bool,
    pub hidden: bool,
    pub available_model_keys: Vec<String>,
    pub available_encoding_configs: Vec<VibeEncodingConfigDto>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub document: ResourceRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_image: Option<ResourceRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ResourceRefDto>,
    pub encodings: Vec<ResourceRefDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct VibeDocumentPageDto {
    pub items: Vec<VibeDocumentEntryDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ImportedVibeDocumentsDto {
    pub entries: Vec<VibeDocumentEntryDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExportedVibeDocumentDto {
    pub file_extension: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EnsuredVibeEncodingDto {
    pub resource: ResourceRefDto,
    pub created: bool,
}
