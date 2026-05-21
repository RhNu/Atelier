#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconStats {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconSubcategorySummary {
    pub name: String,
    pub tag_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconCategorySummary {
    pub name: String,
    pub tag_count: usize,
    pub subcategory_count: usize,
    pub subcategories: Vec<PromptLexiconSubcategorySummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconCatalog {
    pub stats: PromptLexiconStats,
    pub categories: Vec<PromptLexiconCategorySummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconListQuery {
    pub query: String,
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PromptLexiconMatchField {
    Tag,
    PrimaryTranslation,
    Alias,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PromptLexiconMatchRank {
    Exact,
    Prefix,
    Substring,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconEntry {
    pub tag: String,
    pub weight: Option<u64>,
    pub category: String,
    pub subcategory: String,
    pub primary_translation: String,
    pub matched_translation: String,
    pub match_field: PromptLexiconMatchField,
    pub match_rank: PromptLexiconMatchRank,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconSearchResult {
    pub items: Vec<PromptLexiconEntry>,
    pub total: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconListPage {
    pub items: Vec<PromptLexiconEntry>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}
