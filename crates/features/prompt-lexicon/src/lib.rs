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

mod error;
mod model;
mod payload;
mod search;
mod validation;

pub use error::PromptLexiconError;
pub use model::*;
use payload::LexiconFile;
use search::{
    SearchEntry, SearchMatch, compare_browse_entries, compare_search_match, insert_sorted_match,
};
use validation::{
    derive_stats, normalize_catalog_key, normalize_search_text, required_catalog_key,
    slice_checked, validate_payload_ranges,
};

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
