use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::generation::{ImageModelDto, QualityPresetDto};
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
    pub models: Vec<ImageModelDto>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PromptPresetKindDto {
    Main,
    Character,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[ts(tag = "mode", rename_all = "snake_case")]
pub enum PromptPresetBehaviorDto {
    Surround { before: String, after: String },
    Replace { text: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptPresetDto {
    pub preset_id: String,
    pub kind: PromptPresetKindDto,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub order: i32,
    pub prompt_behavior: PromptPresetBehaviorDto,
    pub uc_behavior: PromptPresetBehaviorDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_override: Option<QualityPresetDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uc_preset_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ResourceRefDto>,
    pub models: Vec<ImageModelDto>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct UpsertPromptPresetRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    pub kind: PromptPresetKindDto,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub order: i32,
    pub prompt_behavior: PromptPresetBehaviorDto,
    pub uc_behavior: PromptPresetBehaviorDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_override: Option<QualityPresetDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uc_preset_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<ResourceRefDto>,
    pub models: Vec<ImageModelDto>,
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
    pub models: Vec<ImageModelDto>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ImageModelDto>,
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ListPromptPresetsRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<PromptPresetKindDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ImageModelDto>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptPresetPageDto {
    pub items: Vec<PromptPresetDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeletePromptChunkRequestDto {
    pub chunk_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeletePromptPresetRequestDto {
    pub preset_id: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeletePromptChunkResponseDto {
    pub deleted: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeletePromptPresetResponseDto {
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompilePromptRequestDto {
    pub prompt: String,
    pub model: ImageModelDto,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
}

impl CompilePromptRequestDto {
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            model: ImageModelDto::default(),
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CompileGenerationPromptRequestDto {
    pub model: ImageModelDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_preset_id: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_override: Option<QualityPresetDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uc_preset_override: Option<String>,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LexiconEntityKindDto {
    Tag,
    Artist,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LexiconCategoryDto {
    General,
    Copyright,
    Character,
    Artist,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LexiconContentRatingDto {
    Safe,
    Sensitive,
    Unknown,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LexiconSearchModeDto {
    Lexical,
    Semantic,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LexiconDraftTargetDto {
    Positive,
    Negative,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconCapabilityStatusDto {
    pub lexical_available: bool,
    pub semantic_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconStatsDto {
    pub total_entities: u64,
    pub tag_entities: u64,
    pub artist_entities: u64,
    pub sensitive_entities: u64,
    pub translation_count: u64,
    pub group_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconFacetDto {
    pub value: String,
    pub label: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconGroupSummaryDto {
    pub id: String,
    pub name: String,
    pub member_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconBootstrapDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_version: Option<String>,
    pub status: LexiconCapabilityStatusDto,
    pub stats: LexiconStatsDto,
    pub categories: Vec<LexiconFacetDto>,
    pub groups: Vec<LexiconGroupSummaryDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct LexiconSearchItemDto {
    pub entity_id: u64,
    pub canonical_name: String,
    pub primary_translation: String,
    pub kind: LexiconEntityKindDto,
    pub category: LexiconCategoryDto,
    pub post_count: u64,
    pub rating: LexiconContentRatingDto,
    pub matched_text: String,
    pub match_reason: String,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconCompleteRequestDto {
    pub query: String,
    pub limit: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconSearchFiltersDto {
    pub entity_kinds: Vec<LexiconEntityKindDto>,
    pub categories: Vec<LexiconCategoryDto>,
    pub group_ids: Vec<String>,
    pub ratings: Vec<LexiconContentRatingDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconSearchRequestDto {
    pub query: String,
    pub mode: LexiconSearchModeDto,
    pub filters: LexiconSearchFiltersDto,
    pub selected_entity_ids: Vec<u64>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct LexiconSearchPageDto {
    pub items: Vec<LexiconSearchItemDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LexiconEntityRequestDto {
    pub entity_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LocalizedLexiconTextDto {
    pub locale: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct LexiconRelatedEntityDto {
    pub entity: LexiconSearchItemDto,
    pub relation: String,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct LexiconEntityDetailDto {
    pub entity: LexiconSearchItemDto,
    pub translations: Vec<LocalizedLexiconTextDto>,
    pub aliases: Vec<String>,
    pub wiki: Vec<LocalizedLexiconTextDto>,
    pub groups: Vec<LexiconGroupSummaryDto>,
    pub related: Vec<LexiconRelatedEntityDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct AppendLexiconEntitiesRequestDto {
    pub target: LexiconDraftTargetDto,
    pub entity_ids: Vec<u64>,
}

const fn default_max_depth() -> usize {
    16
}
