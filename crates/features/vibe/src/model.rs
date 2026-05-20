use nai_atelier_resource_catalog::ResourceRef;
use serde_json::Value;

use crate::{VibeClientError, VibeDomainResult, VibeError};

pub type VibeResult<T> = Result<T, VibeClientError>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VibeId(String);

impl VibeId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum VibeModel {
    #[default]
    NaiDiffusion45Full,
    NaiDiffusion45Curated,
    NaiDiffusion4Full,
    NaiDiffusion4Curated,
    NaiDiffusion3,
    NaiDiffusion3Furry,
}

impl VibeModel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NaiDiffusion45Full => "nai-diffusion-4-5-full",
            Self::NaiDiffusion45Curated => "nai-diffusion-4-5-curated",
            Self::NaiDiffusion4Full => "nai-diffusion-4-full",
            Self::NaiDiffusion4Curated => "nai-diffusion-4-curated",
            Self::NaiDiffusion3 => "nai-diffusion-3",
            Self::NaiDiffusion3Furry => "nai-diffusion-3-furry",
        }
    }

    #[must_use]
    pub const fn vibe_model_key(self) -> &'static str {
        match self {
            Self::NaiDiffusion45Full => "v4-5full",
            Self::NaiDiffusion45Curated => "v4-5curated",
            Self::NaiDiffusion4Full => "v4full",
            Self::NaiDiffusion4Curated => "v4curated",
            Self::NaiDiffusion3 => "v3",
            Self::NaiDiffusion3Furry => "v3furry",
        }
    }

    #[must_use]
    pub fn from_vibe_model_key(value: &str) -> Option<Self> {
        match value {
            "v4-5full" => Some(Self::NaiDiffusion45Full),
            "v4-5curated" => Some(Self::NaiDiffusion45Curated),
            "v4full" => Some(Self::NaiDiffusion4Full),
            "v4curated" => Some(Self::NaiDiffusion4Curated),
            "v3" => Some(Self::NaiDiffusion3),
            "v3furry" => Some(Self::NaiDiffusion3Furry),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncodeVibeRequest {
    pub image: String,
    pub information_extracted: f32,
    pub model: VibeModel,
    pub strict_mode: bool,
}

impl Default for EncodeVibeRequest {
    fn default() -> Self {
        Self {
            image: String::new(),
            information_extracted: 1.0,
            model: VibeModel::default(),
            strict_mode: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedVibe {
    pub payload: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeEncodeSettings {
    pub model: VibeModel,
    pub information_extracted: f32,
}

impl VibeEncodeSettings {
    /// Creates validated settings for a `NovelAI` vibe encoding request.
    ///
    /// # Errors
    /// Returns an error when `information_extracted` is outside `[0.01, 1.0]`
    /// or is not finite.
    pub fn new(model: VibeModel, information_extracted: f32) -> VibeDomainResult<Self> {
        if !information_extracted.is_finite()
            || !(0.01_f32..=1.0_f32).contains(&information_extracted)
        {
            return Err(VibeError::invalid_settings(
                "information_extracted must be between 0.01 and 1.0",
            ));
        }
        Ok(Self {
            model,
            information_extracted,
        })
    }

    #[must_use]
    pub fn normalized_information_extracted(&self) -> f32 {
        (self.information_extracted.clamp(0.01, 1.0) * 1000.0).round() / 1000.0
    }

    #[must_use]
    pub fn information_extracted_key(&self) -> String {
        format!("{:.3}", self.normalized_information_extracted())
    }

    #[must_use]
    pub fn cache_key(&self, source: &VibeSourceIdentity) -> String {
        format!(
            "vibe|source_image_sha256|{}|{}|{}",
            source.content_hash,
            self.model.vibe_model_key(),
            self.information_extracted_key()
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VibeSourceIdentity {
    pub content_hash: String,
}

impl VibeSourceIdentity {
    #[must_use]
    pub fn new_sha256(value: impl Into<String>) -> Self {
        Self {
            content_hash: value.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeEncodingConfig {
    pub model: VibeModel,
    pub settings: VibeEncodeSettings,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeEncodingRecord {
    pub vibe_id: VibeId,
    pub source: VibeSourceIdentity,
    pub settings: VibeEncodeSettings,
    pub resource: ResourceRef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeDocumentSummary {
    pub document_id: VibeId,
    pub display_name: String,
    pub has_image: bool,
    pub available_model_keys: Vec<String>,
    pub available_encoding_configs: Vec<VibeEncodingConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VibeDocumentResources {
    pub document: ResourceRef,
    pub source_image: Option<ResourceRef>,
    pub preview: Option<ResourceRef>,
    pub encodings: Vec<ResourceRef>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeDocumentEntry {
    pub summary: VibeDocumentSummary,
    pub resources: VibeDocumentResources,
}

impl VibeDocumentEntry {
    #[must_use]
    pub const fn document_id(&self) -> &VibeId {
        &self.summary.document_id
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeImportedEncoding {
    pub model_key: String,
    pub encoding_key: String,
    pub payload: String,
    pub config: Option<VibeEncodingConfig>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeImportEntry {
    pub summary: VibeDocumentSummary,
    pub official_document: Value,
    pub document_payload: String,
    pub source_image_payload: Option<String>,
    pub preview_payload: Option<String>,
    pub encoding_payloads: Vec<VibeImportedEncoding>,
}

impl VibeImportEntry {
    #[must_use]
    pub fn export_entry(&self) -> VibeExportEntry {
        VibeExportEntry {
            official_document: self.official_document.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VibeExportEntry {
    pub official_document: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeImportDocument {
    pub entries: Vec<VibeImportEntry>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VibeExportFormat {
    Naiv4vibe,
    Naiv4vibebundle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VibeExportDocument {
    pub file_extension: &'static str,
    pub content: String,
}
