use atelier_prompt::{FunctionValue, parse_prompt};
use atelier_resource_catalog::ResourceRef;

use crate::PromptResourceError;
use crate::references::chunk_references_in_text;

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
    /// Parses a chunk key using the same identifier shape accepted by prompt
    /// extension call arguments.
    ///
    /// # Errors
    /// Returns an error when the key is not a single prompt identifier.
    pub fn parse(value: &str) -> Result<Self, PromptResourceError> {
        let source = format!("@chunk({value})");
        let parsed = parse_prompt(&source);
        let ast = parsed.ast();
        let Some(call) = ast.extension_calls().first() else {
            return Err(Self::invalid_key(value));
        };
        if ast.extension_calls().len() != 1 || call.args.len() != 1 {
            return Err(Self::invalid_key(value));
        }
        let arg = &call.args[0];
        if arg.name.is_none()
            && matches!(&arg.value, FunctionValue::Identifier(identifier) if identifier == value)
        {
            Ok(Self(value.to_owned()))
        } else {
            Err(Self::invalid_key(value))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn invalid_key(value: &str) -> PromptResourceError {
        PromptResourceError::invalid_request(format!("invalid chunk key `{value}`"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptChunk {
    pub id: PromptChunkId,
    pub key: PromptChunkKey,
    pub content: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub preview_thumb: Option<ResourceRef>,
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
    pub content: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub preview_thumb: Option<ResourceRef>,
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
