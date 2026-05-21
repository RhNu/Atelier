use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

const LEXICON_SCHEMA: &str = "nai-atelier-prompt-lexicon";
const LEXICON_VERSION: u32 = 1;
const CATEGORY_PRIMARY_PRIORITY: i64 = i64::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptLexiconBuildConfig {
    pub source_dir: PathBuf,
    pub json_dir: PathBuf,
    pub category_order_file: PathBuf,
    pub manifest_file: PathBuf,
    pub output_file: PathBuf,
}

impl PromptLexiconBuildConfig {
    #[must_use]
    pub fn default_for_workspace(workspace_root: impl AsRef<Path>) -> Self {
        let source_dir = workspace_root
            .as_ref()
            .join("assets/prompt-lexicon/sources");
        Self {
            json_dir: source_dir.join("json"),
            category_order_file: source_dir.join("category-order.json"),
            manifest_file: source_dir.join("translation-sources.json"),
            output_file: workspace_root
                .as_ref()
                .join("assets/prompt-lexicon/generated/lexicon.json"),
            source_dir,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptLexiconBuildSummary {
    pub output_file: PathBuf,
    pub size_bytes: u64,
    pub total_tags: u64,
    pub categorized_tags: u64,
    pub uncategorized_tags: u64,
    pub matched_weights: u64,
    pub total_translations: u64,
    pub tags_with_aliases: u64,
}

/// Builds the generated prompt lexicon JSON asset.
///
/// # Errors
/// Returns an error when a source file cannot be read, parsed, or written.
pub fn build_prompt_lexicon(
    config: &PromptLexiconBuildConfig,
) -> Result<PromptLexiconBuildSummary, String> {
    let manifest = load_source_manifest(&config.manifest_file)?;
    let lexicon = merge_lexicon(config, &manifest)?;
    let encoded = serde_json::to_string(&lexicon).map_err(|error| error.to_string())? + "\n";
    if let Some(parent) = config.output_file.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&config.output_file, encoded).map_err(|error| error.to_string())?;
    let size_bytes = fs::metadata(&config.output_file)
        .map_err(|error| error.to_string())?
        .len();
    Ok(PromptLexiconBuildSummary {
        output_file: config.output_file.clone(),
        size_bytes,
        total_tags: lexicon.stats.total_tags,
        categorized_tags: lexicon.stats.categorized_tags,
        uncategorized_tags: lexicon.stats.uncategorized_tags,
        matched_weights: lexicon.stats.matched_weights,
        total_translations: lexicon.stats.total_translations,
        tags_with_aliases: lexicon.stats.tags_with_aliases,
    })
}

/// Verifies that the checked-in generated prompt lexicon is up to date.
///
/// # Errors
/// Returns an error when rebuilding fails or the generated output is stale.
pub fn check_prompt_lexicon(config: &PromptLexiconBuildConfig) -> Result<(), String> {
    let expected = fs::read(&config.output_file).map_err(|error| error.to_string())?;
    let temp_file = std::env::temp_dir().join(format!(
        "nai_atelier_prompt_lexicon_{}_{}.json",
        std::process::id(),
        unique_ms()
    ));
    let mut temp_config = config.clone();
    temp_config.output_file.clone_from(&temp_file);
    build_prompt_lexicon(&temp_config)?;
    let actual = fs::read(&temp_file).map_err(|error| error.to_string())?;
    let _ = fs::remove_file(&temp_file);
    if expected == actual {
        Ok(())
    } else {
        Err("generated prompt lexicon is stale; run `cargo xtask lexicon build`".to_owned())
    }
}

mod category;
mod manifest;
mod output;
mod records;
mod source;
mod util;

use category::{CategoryOrdering, merge_lexicon};
use manifest::{
    ManifestSource, SourceManifest, SourceParser, compare_manifest_source, load_source_manifest,
};
use output::{LexiconFile, finalize_lexicon};
use records::{
    TagRecord, TranslationInput, compare_records_for_export, ensure_tag_record,
    merge_translation_candidate, resolve_translations,
};
use source::ingest_source;
use util::{
    classify_other_bucket, is_valid_tag, normalize_display_text, normalize_tag,
    normalize_tag_display, normalize_text, read_json_value, strip_bom, unique_ms,
};
