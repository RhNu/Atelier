use atelier_generation::{ImageModel, QualityPreset};
use atelier_resource_catalog::ResourceRef;
use atelier_resource_library::{LibraryFolderId, ResourceIdentifier, ResourcePath};

use crate::PromptResourceError;
use crate::references::{chunk_references_in_text, rewrite_chunk_references};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PromptChunkId(String);

impl PromptChunkId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PromptChunkKey(String);

impl PromptChunkKey {
    /// Parses a root-relative resource path used by `$chunk(...)`.
    ///
    /// # Errors
    /// Returns an error when the key is not a canonical resource path.
    pub fn parse(value: &str) -> Result<Self, PromptResourceError> {
        ResourcePath::parse(value)
            .map(|path| Self(path.as_str().to_owned()))
            .map_err(|_| Self::invalid_key(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    /// Returns the final identifier segment of this chunk path.
    ///
    /// # Panics
    /// Panics only if an instance bypasses [`Self::parse`] and violates its
    /// non-empty path invariant.
    pub fn identifier(&self) -> ResourceIdentifier {
        let value = self.0.rsplit('/').next().expect("validated resource path");
        ResourceIdentifier::parse(value).expect("validated resource path segment")
    }

    fn invalid_key(value: &str) -> PromptResourceError {
        PromptResourceError::invalid_request(format!("invalid chunk key `{value}`"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptChunk {
    pub id: PromptChunkId,
    pub key: PromptChunkKey,
    pub folder_id: Option<LibraryFolderId>,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub content: String,
    pub description: Option<String>,
    pub preview_thumb: Option<ResourceRef>,
    pub models: Vec<ImageModel>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl PromptChunk {
    #[must_use]
    pub fn references_chunk(&self, key: &PromptChunkKey) -> bool {
        chunk_references_in_text(&self.content, key)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpsertPromptChunkRequest {
    pub chunk_id: Option<PromptChunkId>,
    pub key: PromptChunkKey,
    pub folder_id: Option<LibraryFolderId>,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub content: String,
    pub description: Option<String>,
    pub preview_thumb: Option<ResourceRef>,
    pub models: Vec<ImageModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletePromptChunkResult {
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkReference {
    pub chunk_id: PromptChunkId,
    pub key: PromptChunkKey,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PromptPresetId(String);

impl PromptPresetId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptPresetKind {
    Main,
    Character,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PromptPresetBehavior {
    Surround { before: String, after: String },
    Replace { text: String },
}

impl PromptPresetBehavior {
    #[must_use]
    pub fn references_chunk(&self, key: &PromptChunkKey) -> bool {
        match self {
            Self::Surround { before, after } => [before, after]
                .into_iter()
                .any(|text| chunk_references_in_text(text, key)),
            Self::Replace { text } => chunk_references_in_text(text, key),
        }
    }

    pub fn rewrite_chunk_references(&mut self, old_key: &PromptChunkKey, new_key: &PromptChunkKey) {
        match self {
            Self::Surround { before, after } => {
                *before = rewrite_chunk_references(before, old_key, new_key);
                *after = rewrite_chunk_references(after, old_key, new_key);
            }
            Self::Replace { text } => {
                *text = rewrite_chunk_references(text, old_key, new_key);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptPreset {
    pub id: PromptPresetId,
    pub kind: PromptPresetKind,
    pub path: ResourcePath,
    pub folder_id: Option<LibraryFolderId>,
    pub identifier: ResourceIdentifier,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub prompt_behavior: PromptPresetBehavior,
    pub uc_behavior: PromptPresetBehavior,
    pub quality_override: Option<QualityPreset>,
    pub uc_preset_override: Option<String>,
    pub preview_thumb: Option<ResourceRef>,
    pub models: Vec<ImageModel>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl PromptPreset {
    #[must_use]
    pub fn references_chunk(&self, key: &PromptChunkKey) -> bool {
        self.prompt_behavior.references_chunk(key) || self.uc_behavior.references_chunk(key)
    }

    pub fn rewrite_chunk_references(&mut self, old_key: &PromptChunkKey, new_key: &PromptChunkKey) {
        self.prompt_behavior
            .rewrite_chunk_references(old_key, new_key);
        self.uc_behavior.rewrite_chunk_references(old_key, new_key);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpsertPromptPresetRequest {
    pub preset_id: Option<PromptPresetId>,
    pub kind: PromptPresetKind,
    pub path: ResourcePath,
    pub folder_id: Option<LibraryFolderId>,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub prompt_behavior: PromptPresetBehavior,
    pub uc_behavior: PromptPresetBehavior,
    pub quality_override: Option<QualityPreset>,
    pub uc_preset_override: Option<String>,
    pub preview_thumb: Option<ResourceRef>,
    pub models: Vec<ImageModel>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletePromptPresetResult {
    pub deleted: bool,
}
