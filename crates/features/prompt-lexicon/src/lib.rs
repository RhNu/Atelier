//! Prompt lexicon feature crate.

use std::cmp::Ordering;
use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;

const EXPECTED_SCHEMA: &str = "nai-atelier-prompt-lexicon";
const EXPECTED_VERSION: u32 = 1;
const SORTED_INSERTION_LIMIT: usize = 512;
const EMBEDDED_PROMPT_LEXICON: &str =
    include_str!("../../../../assets/prompt-lexicon/generated/lexicon.json");

#[derive(Debug, Error)]
pub enum PromptLexiconError {
    #[error("failed to parse prompt lexicon: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("unsupported prompt lexicon schema `{schema}` version {version}")]
    UnsupportedSchema { schema: String, version: u32 },
    #[error("invalid prompt lexicon payload: {0}")]
    InvalidPayload(String),
    #[error("invalid prompt lexicon request: {0}")]
    InvalidRequest(String),
}

#[derive(Clone, Debug)]
pub struct PromptLexicon {
    catalog: PromptLexiconCatalog,
    entries: Vec<SearchEntry>,
    browse_buckets: HashMap<(String, String), Vec<usize>>,
}

impl PromptLexicon {
    /// Loads the generated prompt lexicon bundled with the repository.
    ///
    /// # Errors
    /// Returns an error when the embedded payload is invalid.
    pub fn load_embedded() -> Result<Self, PromptLexiconError> {
        Self::from_json_str(EMBEDDED_PROMPT_LEXICON)
    }

    /// Loads a prompt lexicon from a generated JSON payload.
    ///
    /// # Errors
    /// Returns an error when the JSON is invalid or uses an unsupported schema.
    pub fn from_json_str(payload: &str) -> Result<Self, PromptLexiconError> {
        let payload = serde_json::from_str::<LexiconFile>(payload)?;
        Self::from_payload(payload)
    }

    /// Returns the prompt lexicon catalog tree.
    #[must_use]
    pub fn catalog(&self) -> PromptLexiconCatalog {
        self.catalog.clone()
    }

    /// Lists browse or search results with pagination.
    ///
    /// # Errors
    /// Returns an error when browse mode omits category or subcategory.
    pub fn list(
        &self,
        query: &PromptLexiconListQuery,
    ) -> Result<PromptLexiconListPage, PromptLexiconError> {
        let limit = query.limit.clamp(1, 100);
        let offset = query.offset;
        let normalized_query = normalize_search_text(&query.query);
        if !normalized_query.is_empty() {
            let max_items = offset.saturating_add(limit).min(self.entries.len());
            let (total, matches) = self.sorted_matches(&normalized_query, max_items);
            let items = matches
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(SearchMatch::into_entry)
                .collect();
            return Ok(PromptLexiconListPage {
                items,
                total,
                offset,
                limit,
            });
        }

        let category = required_catalog_key(query.category.as_deref(), "category")?;
        let subcategory = required_catalog_key(query.subcategory.as_deref(), "subcategory")?;
        let bucket = self
            .browse_buckets
            .get(&(category, subcategory))
            .ok_or_else(|| {
                PromptLexiconError::InvalidRequest(
                    "prompt lexicon category or subcategory is invalid".to_owned(),
                )
            })?;
        let total = bucket.len();
        let items = bucket
            .iter()
            .skip(offset)
            .take(limit)
            .map(|index| self.entries[*index].to_browse_entry())
            .collect();
        Ok(PromptLexiconListPage {
            items,
            total,
            offset,
            limit,
        })
    }

    /// Searches the lexicon by tag, primary translation, or alias.
    #[must_use]
    pub fn search(&self, query: &str, limit: usize) -> PromptLexiconSearchResult {
        let limit = limit.clamp(1, 50);
        let normalized_query = normalize_search_text(query);
        if normalized_query.is_empty() {
            return PromptLexiconSearchResult {
                total: 0,
                items: Vec::new(),
            };
        }
        let (total, matches) = self.sorted_matches(&normalized_query, limit);
        PromptLexiconSearchResult {
            total,
            items: matches
                .into_iter()
                .take(limit)
                .map(SearchMatch::into_entry)
                .collect(),
        }
    }

    fn from_payload(payload: LexiconFile) -> Result<Self, PromptLexiconError> {
        if payload.schema != EXPECTED_SCHEMA || payload.version != EXPECTED_VERSION {
            return Err(PromptLexiconError::UnsupportedSchema {
                schema: payload.schema,
                version: payload.version,
            });
        }

        validate_payload_ranges(&payload)?;
        let stats = match payload.stats.clone() {
            Some(stats) => stats.into(),
            None => derive_stats(&payload)?,
        };
        let mut entries = Vec::new();
        let mut categories = Vec::with_capacity(payload.categories.len());
        let mut browse_buckets = HashMap::new();
        for (category_index, category) in payload.categories.iter().enumerate() {
            let subcategories = slice_checked(
                &payload.subcategories,
                category.subcategory_start,
                category.subcategory_count,
                "category subcategory range",
            )?;
            let mut subcategory_summaries = Vec::with_capacity(subcategories.len());
            let mut category_tag_count = 0usize;
            for subcategory in subcategories {
                if subcategory.category_index != category_index {
                    return Err(PromptLexiconError::InvalidPayload(
                        "subcategory category_index does not match parent".to_owned(),
                    ));
                }
                let tags = slice_checked(
                    &payload.tags,
                    subcategory.tag_start,
                    subcategory.tag_count,
                    "subcategory tag range",
                )?;
                let mut entry_indices = Vec::with_capacity(tags.len());
                for tag in tags {
                    let translations = slice_checked(
                        &payload.translations,
                        tag.translation_start,
                        tag.translation_count,
                        "tag translation range",
                    )?;
                    let Some(primary_translation) = translations.first() else {
                        return Err(PromptLexiconError::InvalidPayload(
                            "tag is missing a primary translation".to_owned(),
                        ));
                    };
                    let entry_index = entries.len();
                    entries.push(SearchEntry::new(
                        tag.tag.clone(),
                        tag.weight,
                        category.name.clone(),
                        subcategory.name.clone(),
                        primary_translation.clone(),
                        translations[1..].to_vec(),
                    ));
                    entry_indices.push(entry_index);
                }
                entry_indices.sort_by(|left, right| {
                    compare_browse_entries(&entries[*left], &entries[*right])
                });
                category_tag_count += tags.len();
                subcategory_summaries.push(PromptLexiconSubcategorySummary {
                    name: subcategory.name.clone(),
                    tag_count: tags.len(),
                });
                browse_buckets.insert(
                    (
                        normalize_catalog_key(&category.name),
                        normalize_catalog_key(&subcategory.name),
                    ),
                    entry_indices,
                );
            }
            categories.push(PromptLexiconCategorySummary {
                name: category.name.clone(),
                tag_count: category_tag_count,
                subcategory_count: subcategory_summaries.len(),
                subcategories: subcategory_summaries,
            });
        }

        Ok(Self {
            catalog: PromptLexiconCatalog { stats, categories },
            entries,
            browse_buckets,
        })
    }

    fn sorted_matches(&self, query: &str, max_items: usize) -> (usize, Vec<SearchMatch<'_>>) {
        if max_items > SORTED_INSERTION_LIMIT {
            let mut matches = self
                .entries
                .iter()
                .filter_map(|entry| entry.match_query(query))
                .collect::<Vec<_>>();
            matches.sort_by(compare_search_match);
            return (matches.len(), matches);
        }

        let mut total = 0usize;
        let mut matches = Vec::with_capacity(max_items.min(self.entries.len()));
        for entry in &self.entries {
            let Some(candidate) = entry.match_query(query) else {
                continue;
            };
            total += 1;
            insert_sorted_match(&mut matches, candidate, max_items);
        }
        (total, matches)
    }
}

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

#[derive(Clone, Debug)]
struct SearchEntry {
    tag: String,
    weight: Option<u64>,
    category: String,
    subcategory: String,
    primary_translation: String,
    aliases: Vec<String>,
    normalized_tag: String,
    normalized_primary_translation: String,
    normalized_aliases: Vec<String>,
}

impl SearchEntry {
    fn new(
        tag: String,
        weight: Option<u64>,
        category: String,
        subcategory: String,
        primary_translation: String,
        aliases: Vec<String>,
    ) -> Self {
        Self {
            normalized_tag: normalize_search_text(&tag),
            normalized_primary_translation: normalize_search_text(&primary_translation),
            normalized_aliases: aliases
                .iter()
                .map(|item| normalize_search_text(item))
                .collect(),
            tag,
            weight,
            category,
            subcategory,
            primary_translation,
            aliases,
        }
    }

    fn match_query(&self, query: &str) -> Option<SearchMatch<'_>> {
        if query.is_empty() {
            return None;
        }
        for rank in [
            PromptLexiconMatchRank::Exact,
            PromptLexiconMatchRank::Prefix,
            PromptLexiconMatchRank::Substring,
        ] {
            if matches_query(&self.normalized_tag, query, rank) {
                return Some(SearchMatch::new(
                    self,
                    PromptLexiconMatchField::Tag,
                    rank,
                    self.primary_translation.as_str(),
                ));
            }
            if matches_query(&self.normalized_primary_translation, query, rank) {
                return Some(SearchMatch::new(
                    self,
                    PromptLexiconMatchField::PrimaryTranslation,
                    rank,
                    self.primary_translation.as_str(),
                ));
            }
            if let Some(alias) =
                find_alias_match(query, &self.aliases, &self.normalized_aliases, rank)
            {
                return Some(SearchMatch::new(
                    self,
                    PromptLexiconMatchField::Alias,
                    rank,
                    alias,
                ));
            }
        }
        None
    }

    fn to_browse_entry(&self) -> PromptLexiconEntry {
        PromptLexiconEntry {
            tag: self.tag.clone(),
            weight: self.weight,
            category: self.category.clone(),
            subcategory: self.subcategory.clone(),
            primary_translation: self.primary_translation.clone(),
            matched_translation: self.primary_translation.clone(),
            match_field: PromptLexiconMatchField::Tag,
            match_rank: PromptLexiconMatchRank::Substring,
        }
    }
}

struct SearchMatch<'a> {
    entry: &'a SearchEntry,
    match_field: PromptLexiconMatchField,
    match_rank: PromptLexiconMatchRank,
    matched_translation: &'a str,
}

impl<'a> SearchMatch<'a> {
    const fn new(
        entry: &'a SearchEntry,
        match_field: PromptLexiconMatchField,
        match_rank: PromptLexiconMatchRank,
        matched_translation: &'a str,
    ) -> Self {
        Self {
            entry,
            match_field,
            match_rank,
            matched_translation,
        }
    }

    fn into_entry(self) -> PromptLexiconEntry {
        PromptLexiconEntry {
            tag: self.entry.tag.clone(),
            weight: self.entry.weight,
            category: self.entry.category.clone(),
            subcategory: self.entry.subcategory.clone(),
            primary_translation: self.entry.primary_translation.clone(),
            matched_translation: self.matched_translation.to_owned(),
            match_field: self.match_field,
            match_rank: self.match_rank,
        }
    }
}

#[derive(Debug, Deserialize)]
struct LexiconFile {
    schema: String,
    version: u32,
    #[serde(default)]
    sources: Vec<serde_json::Value>,
    categories: Vec<LexiconCategory>,
    subcategories: Vec<LexiconSubcategory>,
    tags: Vec<LexiconTag>,
    translations: Vec<String>,
    #[serde(default)]
    stats: Option<PromptLexiconStatsPayload>,
}

#[derive(Clone, Debug, Deserialize)]
struct PromptLexiconStatsPayload {
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
struct LexiconCategory {
    name: String,
    subcategory_start: usize,
    subcategory_count: usize,
}

#[derive(Debug, Deserialize)]
struct LexiconSubcategory {
    name: String,
    category_index: usize,
    tag_start: usize,
    tag_count: usize,
}

#[derive(Debug, Deserialize)]
struct LexiconTag {
    tag: String,
    #[serde(default)]
    weight: Option<u64>,
    translation_start: usize,
    translation_count: usize,
}

fn compare_search_match(left: &SearchMatch<'_>, right: &SearchMatch<'_>) -> Ordering {
    rank_priority(left.match_rank)
        .cmp(&rank_priority(right.match_rank))
        .then_with(|| field_priority(left.match_field).cmp(&field_priority(right.match_field)))
        .then_with(|| {
            right
                .entry
                .weight
                .unwrap_or(0)
                .cmp(&left.entry.weight.unwrap_or(0))
        })
        .then_with(|| left.entry.tag.cmp(&right.entry.tag))
}

fn insert_sorted_match<'a>(
    matches: &mut Vec<SearchMatch<'a>>,
    candidate: SearchMatch<'a>,
    limit: usize,
) {
    if limit == 0 {
        return;
    }
    let position = matches
        .binary_search_by(|probe| compare_search_match(probe, &candidate))
        .unwrap_or_else(|position| position);
    if position >= limit {
        return;
    }
    matches.insert(position, candidate);
    if matches.len() > limit {
        matches.pop();
    }
}

fn compare_browse_entries(left: &SearchEntry, right: &SearchEntry) -> Ordering {
    right
        .weight
        .unwrap_or(0)
        .cmp(&left.weight.unwrap_or(0))
        .then_with(|| left.tag.cmp(&right.tag))
}

const fn rank_priority(rank: PromptLexiconMatchRank) -> u8 {
    match rank {
        PromptLexiconMatchRank::Exact => 0,
        PromptLexiconMatchRank::Prefix => 1,
        PromptLexiconMatchRank::Substring => 2,
    }
}

const fn field_priority(field: PromptLexiconMatchField) -> u8 {
    match field {
        PromptLexiconMatchField::Tag => 0,
        PromptLexiconMatchField::PrimaryTranslation => 1,
        PromptLexiconMatchField::Alias => 2,
    }
}

fn matches_query(value: &str, query: &str, rank: PromptLexiconMatchRank) -> bool {
    match rank {
        PromptLexiconMatchRank::Exact => value == query,
        PromptLexiconMatchRank::Prefix => value.starts_with(query),
        PromptLexiconMatchRank::Substring => value.contains(query),
    }
}

fn find_alias_match<'a>(
    query: &str,
    aliases: &'a [String],
    normalized_aliases: &[String],
    rank: PromptLexiconMatchRank,
) -> Option<&'a str> {
    aliases
        .iter()
        .zip(normalized_aliases)
        .find_map(|(alias, normalized)| {
            matches_query(normalized, query, rank).then_some(alias.as_str())
        })
}

fn normalize_search_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_catalog_key(value: &str) -> String {
    value.trim().to_lowercase()
}

fn required_catalog_key(value: Option<&str>, name: &str) -> Result<String, PromptLexiconError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_catalog_key)
        .ok_or_else(|| {
            PromptLexiconError::InvalidRequest(format!(
                "prompt lexicon browse mode requires {name}"
            ))
        })
}

fn slice_checked<'a, T>(
    values: &'a [T],
    start: usize,
    count: usize,
    label: &str,
) -> Result<&'a [T], PromptLexiconError> {
    let end = start.checked_add(count).ok_or_else(|| {
        PromptLexiconError::InvalidPayload(format!("{label} is outside the payload"))
    })?;
    values.get(start..end).ok_or_else(|| {
        PromptLexiconError::InvalidPayload(format!("{label} is outside the payload"))
    })
}

fn validate_payload_ranges(payload: &LexiconFile) -> Result<(), PromptLexiconError> {
    for category in &payload.categories {
        slice_checked(
            &payload.subcategories,
            category.subcategory_start,
            category.subcategory_count,
            "category subcategory range",
        )?;
    }
    for subcategory in &payload.subcategories {
        slice_checked(
            &payload.tags,
            subcategory.tag_start,
            subcategory.tag_count,
            "subcategory tag range",
        )?;
    }
    for tag in &payload.tags {
        slice_checked(
            &payload.translations,
            tag.translation_start,
            tag.translation_count,
            "tag translation range",
        )?;
    }
    Ok(())
}

fn derive_stats(payload: &LexiconFile) -> Result<PromptLexiconStats, PromptLexiconError> {
    let total_tags = payload.tags.len() as u64;
    let total_translations = payload.translations.len() as u64;
    let matched_weights = payload
        .tags
        .iter()
        .filter(|tag| tag.weight.is_some())
        .count() as u64;
    let tags_with_aliases = payload
        .tags
        .iter()
        .filter(|tag| tag.translation_count > 1)
        .count() as u64;
    let max_aliases_per_tag = payload
        .tags
        .iter()
        .map(|tag| tag.translation_count.saturating_sub(1) as u64)
        .max()
        .unwrap_or(0);
    let uncategorized_tags = payload
        .categories
        .iter()
        .filter(|category| normalize_catalog_key(&category.name) == "other")
        .map(|category| -> Result<u64, PromptLexiconError> {
            let subcategories = slice_checked(
                &payload.subcategories,
                category.subcategory_start,
                category.subcategory_count,
                "category subcategory range",
            )?;
            Ok(subcategories
                .iter()
                .map(|subcategory| subcategory.tag_count as u64)
                .sum::<u64>())
        })
        .sum::<Result<u64, _>>()?;
    Ok(PromptLexiconStats {
        total_tags,
        categorized_tags: total_tags.saturating_sub(uncategorized_tags),
        uncategorized_tags,
        matched_weights,
        total_translations,
        tags_with_aliases,
        max_aliases_per_tag,
        source_count: payload.sources.len() as u64,
        manifest_version: 1,
        primary_from_category_json: 0,
        primary_from_manifest_sources: 0,
        primary_fallback_to_tag: 0,
    })
}
