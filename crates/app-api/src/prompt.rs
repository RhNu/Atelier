use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::resource::ResourceRefDto;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptChunkDto {
    pub chunk_id: String,
    pub key: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ResourceRefDto>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct UpsertPromptChunkRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
    pub key: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ResourceRefDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GetPromptChunkRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ListPromptChunksRequestDto {
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptChunkPageDto {
    pub items: Vec<PromptChunkDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeletePromptChunkRequestDto {
    pub chunk_id: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeletePromptChunkResponseDto {
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompilePromptRequestDto {
    pub prompt: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

impl CompilePromptRequestDto {
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_depth: default_max_depth(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompiledPromptDto {
    pub expanded_prompt: String,
    pub trace: PromptTraceDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompileGenerationCharacterPromptDto {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompileGenerationPromptRequestDto {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub characters: Vec<CompileGenerationCharacterPromptDto>,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompiledGenerationCharacterPromptDto {
    pub prompt: CompiledPromptDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<CompiledPromptDto>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompiledGenerationPromptDto {
    pub prompt: CompiledPromptDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<CompiledPromptDto>,
    pub characters: Vec<CompiledGenerationCharacterPromptDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptTraceDto {
    pub raw_prompt: String,
    pub expanded_prompt: String,
    pub function_calls: Vec<PromptFunctionTraceEntryDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptFunctionTraceEntryDto {
    pub function_name: String,
    pub raw_call: String,
    pub resolved_arguments: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_text: Option<String>,
    pub depth: usize,
    pub call_chain: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconCatalogDto {
    pub stats: PromptLexiconStatsDto,
    pub categories: Vec<PromptLexiconCategorySummaryDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconStatsDto {
    pub total_tags: u64,
    pub categorized_tags: u64,
    pub uncategorized_tags: u64,
    pub matched_weights: u64,
    pub total_translations: u64,
    pub tags_with_aliases: u64,
    pub max_aliases_per_tag: u64,
    pub source_count: u64,
    pub manifest_version: u32,
    pub primary_from_category_json: u64,
    pub primary_from_manifest_sources: u64,
    pub primary_fallback_to_tag: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconCategorySummaryDto {
    pub name: String,
    pub tag_count: usize,
    pub subcategory_count: usize,
    pub subcategories: Vec<PromptLexiconSubcategorySummaryDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconSubcategorySummaryDto {
    pub name: String,
    pub tag_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconListQueryDto {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconSearchQueryDto {
    pub query: String,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconPageDto {
    pub items: Vec<PromptLexiconEntryDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptLexiconEntryDto {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u64>,
    pub category: String,
    pub subcategory: String,
    pub primary_translation: String,
    pub matched_translation: String,
    pub match_field: String,
    pub match_rank: String,
}

const fn default_max_depth() -> usize {
    16
}
