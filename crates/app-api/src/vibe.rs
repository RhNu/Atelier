use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::resource::ResourceRefDto;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum VibeModelDto {
    #[default]
    #[serde(rename = "nai-diffusion-4-5-full")]
    NaiDiffusion45Full,
    #[serde(rename = "nai-diffusion-4-5-curated")]
    NaiDiffusion45Curated,
    #[serde(rename = "nai-diffusion-4-full")]
    NaiDiffusion4Full,
    #[serde(rename = "nai-diffusion-4-curated")]
    NaiDiffusion4Curated,
    #[serde(rename = "nai-diffusion-3")]
    NaiDiffusion3,
    #[serde(rename = "nai-diffusion-3-furry")]
    NaiDiffusion3Furry,
}

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
    pub model: VibeModelDto,
    pub information_extracted: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct VibeEncodingConfigDto {
    pub model: VibeModelDto,
    pub information_extracted: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ListVibeDocumentsRequestDto {
    pub offset: usize,
    pub limit: usize,
}

impl Default for ListVibeDocumentsRequestDto {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
        }
    }
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
    pub available_model_keys: Vec<String>,
    pub available_encoding_configs: Vec<VibeEncodingConfigDto>,
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
