//! Prompt lexicon feature crate.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::Deserialize;
use thiserror::Error;

const EXPECTED_SCHEMA: &str = "atelier-prompt-lexicon";
const EXPECTED_VERSION: u32 = 1;
const SORTED_INSERTION_LIMIT: usize = 512;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const EMBEDDED_PROMPT_LEXICON: &str =
    include_str!("../../../../assets/prompt-lexicon/generated/lexicon.json");
static SHARED_EMBEDDED_PROMPT_LEXICON: OnceLock<Arc<PromptLexicon>> = OnceLock::new();
static SHARED_EMBEDDED_PROMPT_LEXICON_LOAD: Mutex<()> = Mutex::new(());

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
    derive_stats, normalize_catalog_key, normalize_search_text, slice_checked,
    validate_payload_ranges,
};

#[derive(Clone, Debug)]
pub struct PromptLexicon {
    catalog: PromptLexiconCatalog,
    entries: Vec<SearchEntry>,
    all_browse_entries: Vec<usize>,
    category_browse_buckets: HashMap<String, Vec<usize>>,
    browse_buckets: HashMap<(String, String), Vec<usize>>,
    search_trigrams: OnceLock<HashMap<u64, Vec<u32>>>,
}

impl PromptLexicon {
    /// Loads the generated prompt lexicon bundled with the repository.
    ///
    /// # Errors
    /// Returns an error when the embedded payload is invalid.
    pub fn load_embedded() -> Result<Self, PromptLexiconError> {
        Self::from_json_str(EMBEDDED_PROMPT_LEXICON)
    }

    /// Loads the generated prompt lexicon once and shares it across workspace sessions.
    ///
    /// # Errors
    /// Returns an error when the embedded payload is invalid.
    pub fn load_embedded_shared() -> Result<Arc<Self>, PromptLexiconError> {
        if let Some(lexicon) = SHARED_EMBEDDED_PROMPT_LEXICON.get() {
            return Ok(Arc::clone(lexicon));
        }
        let _load_guard = SHARED_EMBEDDED_PROMPT_LEXICON_LOAD.lock().map_err(|_| {
            PromptLexiconError::InvalidPayload(
                "shared prompt lexicon load state is unavailable".to_owned(),
            )
        })?;
        if let Some(lexicon) = SHARED_EMBEDDED_PROMPT_LEXICON.get() {
            return Ok(Arc::clone(lexicon));
        }
        let lexicon = Arc::new(Self::load_embedded()?);
        Ok(Arc::clone(
            SHARED_EMBEDDED_PROMPT_LEXICON.get_or_init(|| lexicon),
        ))
    }

    /// Loads a prompt lexicon from a generated JSON payload.
    ///
    /// # Errors
    /// Returns an error when the JSON is invalid or uses an unsupported schema.
    pub fn from_json_str(payload: &str) -> Result<Self, PromptLexiconError> {
        let payload = serde_json::from_str::<LexiconFile>(payload)?;
        Self::from_payload(&payload)
    }

    /// Returns the prompt lexicon catalog tree.
    #[must_use]
    pub fn catalog(&self) -> PromptLexiconCatalog {
        self.catalog.clone()
    }

    /// Lists browse or search results with pagination.
    ///
    /// # Errors
    /// Returns an error when the requested browse category or subcategory is invalid.
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

        let category = query
            .category
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(normalize_catalog_key);
        let subcategory = query
            .subcategory
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(normalize_catalog_key);
        let bucket = match (category, subcategory) {
            (None, None) => &self.all_browse_entries,
            (Some(category), None) => {
                self.category_browse_buckets.get(&category).ok_or_else(|| {
                    PromptLexiconError::InvalidRequest(
                        "prompt lexicon category is invalid".to_owned(),
                    )
                })?
            }
            (Some(category), Some(subcategory)) => self
                .browse_buckets
                .get(&(category, subcategory))
                .ok_or_else(|| {
                    PromptLexiconError::InvalidRequest(
                        "prompt lexicon category or subcategory is invalid".to_owned(),
                    )
                })?,
            (None, Some(_)) => {
                return Err(PromptLexiconError::InvalidRequest(
                    "prompt lexicon subcategory requires category".to_owned(),
                ));
            }
        };
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

    /// Builds the substring search index ahead of the first interactive query.
    pub fn warm_search_index(&self) {
        self.search_trigrams
            .get_or_init(|| build_search_trigram_index(&self.entries));
    }

    fn from_payload(payload: &LexiconFile) -> Result<Self, PromptLexiconError> {
        validate_schema(payload)?;
        validate_payload_ranges(payload)?;
        let stats = match payload.stats.clone() {
            Some(stats) => stats.into(),
            None => derive_stats(payload)?,
        };
        let mut entries = Vec::new();
        let mut categories = Vec::with_capacity(payload.categories.len());
        let mut category_browse_buckets = HashMap::new();
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
            let mut category_entry_indices = Vec::new();
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
                sort_browse_indices(&entries, &mut entry_indices);
                category_entry_indices.extend(entry_indices.iter().copied());
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
            sort_browse_indices(&entries, &mut category_entry_indices);
            let category_key = normalize_catalog_key(&category.name);
            category_browse_buckets.insert(category_key, category_entry_indices);
            categories.push(PromptLexiconCategorySummary {
                name: category.name.clone(),
                tag_count: category_tag_count,
                subcategory_count: subcategory_summaries.len(),
                subcategories: subcategory_summaries,
            });
        }

        let max_searchable_entries = usize::try_from(u32::MAX).unwrap_or(usize::MAX);
        if entries.len() > max_searchable_entries {
            return Err(PromptLexiconError::InvalidPayload(
                "prompt lexicon contains too many searchable entries".to_owned(),
            ));
        }
        let mut all_browse_entries = (0..entries.len()).collect::<Vec<_>>();
        sort_browse_indices(&entries, &mut all_browse_entries);
        Ok(Self {
            catalog: PromptLexiconCatalog { stats, categories },
            entries,
            all_browse_entries,
            category_browse_buckets,
            browse_buckets,
            search_trigrams: OnceLock::new(),
        })
    }

    fn sorted_matches(&self, query: &str, max_items: usize) -> (usize, Vec<SearchMatch<'_>>) {
        let candidates: Box<dyn Iterator<Item = &SearchEntry> + '_> =
            match first_trigram_hash(query) {
                Some(hash) => Box::new(
                    self.search_trigrams
                        .get_or_init(|| build_search_trigram_index(&self.entries))
                        .get(&hash)
                        .into_iter()
                        .flatten()
                        .filter_map(|index| {
                            usize::try_from(*index)
                                .ok()
                                .and_then(|index| self.entries.get(index))
                        }),
                ),
                None => Box::new(self.entries.iter()),
            };
        if max_items > SORTED_INSERTION_LIMIT {
            let mut matches = candidates
                .filter_map(|entry| entry.match_query(query))
                .collect::<Vec<_>>();
            matches.sort_by(compare_search_match);
            return (matches.len(), matches);
        }

        let mut total = 0usize;
        let mut matches = Vec::with_capacity(max_items.min(self.entries.len()));
        for entry in candidates {
            let Some(candidate) = entry.match_query(query) else {
                continue;
            };
            total += 1;
            insert_sorted_match(&mut matches, candidate, max_items);
        }
        (total, matches)
    }
}

fn validate_schema(payload: &LexiconFile) -> Result<(), PromptLexiconError> {
    if payload.schema == EXPECTED_SCHEMA && payload.version == EXPECTED_VERSION {
        return Ok(());
    }
    Err(PromptLexiconError::UnsupportedSchema {
        schema: payload.schema.clone(),
        version: payload.version,
    })
}

fn sort_browse_indices(entries: &[SearchEntry], indices: &mut [usize]) {
    indices.sort_by(|left, right| compare_browse_entries(&entries[*left], &entries[*right]));
}

fn build_search_trigram_index(entries: &[SearchEntry]) -> HashMap<u64, Vec<u32>> {
    let mut index = HashMap::<u64, Vec<u32>>::new();
    let mut hashes = Vec::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        hashes.clear();
        for value in entry.normalized_values() {
            record_trigram_hashes(value, &mut hashes);
        }
        hashes.sort_unstable();
        hashes.dedup();
        let Ok(entry_index) = u32::try_from(entry_index) else {
            break;
        };
        for hash in &hashes {
            index.entry(*hash).or_default().push(entry_index);
        }
    }
    index
}

fn record_trigram_hashes(value: &str, hashes: &mut Vec<u64>) {
    let mut chars = value.chars();
    let (Some(mut first), Some(mut second), Some(mut third)) =
        (chars.next(), chars.next(), chars.next())
    else {
        return;
    };
    hashes.push(hash_chars([first, second, third]));
    for next in chars {
        first = second;
        second = third;
        third = next;
        hashes.push(hash_chars([first, second, third]));
    }
}

fn first_trigram_hash(value: &str) -> Option<u64> {
    let mut chars = value.chars();
    Some(hash_chars([chars.next()?, chars.next()?, chars.next()?]))
}

fn hash_chars(chars: [char; 3]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for character in chars {
        let mut encoded = [0_u8; 4];
        for byte in character.encode_utf8(&mut encoded).as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash
}
