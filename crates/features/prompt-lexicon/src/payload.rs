use super::{Deserialize, PromptLexiconStats};

#[derive(Debug, Deserialize)]
pub struct LexiconFile {
    pub(super) schema: String,
    pub(super) version: u32,
    #[serde(default)]
    pub(super) sources: Vec<serde_json::Value>,
    pub(super) categories: Vec<LexiconCategory>,
    pub(super) subcategories: Vec<LexiconSubcategory>,
    pub(super) tags: Vec<LexiconTag>,
    pub(super) translations: Vec<String>,
    #[serde(default)]
    pub(super) stats: Option<PromptLexiconStatsPayload>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PromptLexiconStatsPayload {
    total_tags: u64,
    categorized_tags: u64,
    uncategorized_tags: u64,
    matched_weights: u64,
    total_translations: u64,
    tags_with_aliases: u64,
    max_aliases_per_tag: u64,
    source_count: u64,
    manifest_version: u32,
    primary_from_category_json: u64,
    primary_from_manifest_sources: u64,
    primary_fallback_to_tag: u64,
}

impl From<PromptLexiconStatsPayload> for PromptLexiconStats {
    fn from(value: PromptLexiconStatsPayload) -> Self {
        Self {
            total_tags: value.total_tags,
            categorized_tags: value.categorized_tags,
            uncategorized_tags: value.uncategorized_tags,
            matched_weights: value.matched_weights,
            total_translations: value.total_translations,
            tags_with_aliases: value.tags_with_aliases,
            max_aliases_per_tag: value.max_aliases_per_tag,
            source_count: value.source_count,
            manifest_version: value.manifest_version,
            primary_from_category_json: value.primary_from_category_json,
            primary_from_manifest_sources: value.primary_from_manifest_sources,
            primary_fallback_to_tag: value.primary_fallback_to_tag,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LexiconCategory {
    pub(super) name: String,
    pub(super) subcategory_start: usize,
    pub(super) subcategory_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct LexiconSubcategory {
    pub(super) name: String,
    pub(super) category_index: usize,
    pub(super) tag_start: usize,
    pub(super) tag_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct LexiconTag {
    pub(super) tag: String,
    #[serde(default)]
    pub(super) weight: Option<u64>,
    pub(super) translation_start: usize,
    pub(super) translation_count: usize,
}
