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

fn merge_lexicon(
    config: &PromptLexiconBuildConfig,
    manifest: &SourceManifest,
) -> Result<LexiconFile, String> {
    let mut records = HashMap::new();
    let ordering =
        load_categorized_json(&config.json_dir, &config.category_order_file, &mut records)?;
    let mut sources = manifest.sources.clone();
    sources.sort_by(compare_manifest_source);
    for source in &sources {
        ingest_source(source, &mut records)?;
    }
    Ok(finalize_lexicon(records, manifest, ordering))
}

fn load_categorized_json(
    json_dir: &Path,
    category_order_file: &Path,
    records: &mut HashMap<String, TagRecord>,
) -> Result<CategoryOrdering, String> {
    if !json_dir.exists() {
        return Ok(CategoryOrdering::default());
    }
    let mut files = fs::read_dir(json_dir)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    files.sort_by_key(std::fs::DirEntry::file_name);
    let mut sources = BTreeMap::new();
    for entry in files {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "json") {
            continue;
        }
        let category = path
            .file_stem()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("invalid category file name: {}", path.display()))?
            .to_owned();
        let raw = read_json_value(&path)?;
        let Some(_groups) = raw.as_object() else {
            continue;
        };
        sources.insert(category, CategorizedJsonSource { path, raw });
    }

    let mut ordering = load_category_order(category_order_file)?.unwrap_or_else(|| {
        let mut category_order = sources.keys().cloned().collect::<Vec<_>>();
        category_order.sort();
        CategoryOrdering {
            category_order,
            subcategory_order_by_category: BTreeMap::new(),
        }
    });
    append_missing_names(&mut ordering.category_order, sources.keys().cloned());

    for category in &ordering.category_order {
        let Some(source) = sources.get(category) else {
            continue;
        };
        let Some(groups) = source.raw.as_object() else {
            continue;
        };
        let mut subcategories = ordering
            .subcategory_order_by_category
            .remove(category)
            .unwrap_or_default();
        append_missing_names(&mut subcategories, groups.keys().cloned());
        ordering
            .subcategory_order_by_category
            .insert(category.clone(), subcategories.clone());
        for subcategory in subcategories {
            let Some(tags) = groups
                .get(&subcategory)
                .and_then(serde_json::Value::as_object)
            else {
                continue;
            };
            for (tag, translation) in tags {
                if !is_valid_tag(tag) {
                    continue;
                }
                let record = ensure_tag_record(records, tag);
                if record.category.is_none() {
                    record.category = Some(category.clone());
                    record.subcategory = Some(subcategory.clone());
                }
                if let Some(text) = translation.as_str() {
                    merge_translation_candidate(
                        record,
                        text,
                        TranslationInput::primary(
                            CATEGORY_PRIMARY_PRIORITY,
                            format!("categorized_json:{}", source.path.display()),
                        ),
                    );
                }
            }
        }
    }
    Ok(ordering)
}

fn load_category_order(path: &Path) -> Result<Option<CategoryOrdering>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw_text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let raw: CategoryOrderFile =
        serde_json::from_str(&strip_bom(&raw_text)).map_err(|error| error.to_string())?;
    let mut category_order = Vec::with_capacity(raw.categories.len());
    let mut subcategory_order_by_category = BTreeMap::new();
    for category in raw.categories {
        let name = category.name.trim().to_owned();
        if name.is_empty() {
            continue;
        }
        category_order.push(name.clone());
        subcategory_order_by_category.insert(name, category.subcategories);
    }
    Ok(Some(CategoryOrdering {
        category_order,
        subcategory_order_by_category,
    }))
}

fn append_missing_names(existing: &mut Vec<String>, names: impl Iterator<Item = String>) {
    let mut seen = existing.iter().cloned().collect::<HashSet<_>>();
    let mut missing = names
        .filter(|name| seen.insert(name.clone()))
        .collect::<Vec<_>>();
    missing.sort();
    existing.extend(missing);
}

fn ingest_source(
    source: &ManifestSource,
    records: &mut HashMap<String, TagRecord>,
) -> Result<(), String> {
    let raw_content = fs::read_to_string(&source.path).map_err(|error| error.to_string())?;
    let content = strip_bom(&raw_content);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());
    for row in reader.records() {
        let columns = row.map_err(|error| error.to_string())?;
        let Some(row) = parse_source_row(source, &columns) else {
            continue;
        };
        if !is_valid_tag(&row.tag) {
            continue;
        }
        let record = ensure_tag_record(records, &row.tag);
        if source.parser == SourceParser::Weighted {
            record.tag = normalize_tag_display(&row.tag);
        }
        if let Some(weight) = row.weight
            && record.weight.is_none_or(|existing| weight > existing)
        {
            record.weight = Some(weight);
        }
        for translation in row.translations {
            merge_translation_candidate(
                record,
                &translation.text,
                TranslationInput {
                    priority: source.priority,
                    allow_primary: translation.allow_primary
                        && source.allow_primary
                        && !source.alias_only,
                    source_id: source.id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn parse_source_row(source: &ManifestSource, columns: &csv::StringRecord) -> Option<SourceRow> {
    match source.parser {
        SourceParser::Weighted => parse_weighted_csv_row(columns),
        SourceParser::Simple => parse_simple_csv_row(source, columns),
        SourceParser::Reversed => parse_reversed_csv_row(source, columns),
        SourceParser::Github => parse_github_csv_row(source, columns),
        SourceParser::Alias => parse_alias_csv_row(columns),
    }
}

fn parse_weighted_csv_row(columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() < 3 {
        return None;
    }
    let tag = columns.get(0)?;
    if normalize_tag(tag) == "tag" {
        return None;
    }
    let weight = columns.get(1)?.parse::<u64>().unwrap_or(0);
    let translation = columns.get(2)?;
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: Some(weight),
        translations: translation_items(translation, true),
    })
}

fn parse_simple_csv_row(source: &ManifestSource, columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() < 2 {
        return None;
    }
    let tag = columns.get(0)?;
    if normalize_tag(tag) == "tag" {
        return None;
    }
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: translation_items(columns.get(1)?, source.allow_primary),
    })
}

fn parse_reversed_csv_row(
    source: &ManifestSource,
    columns: &csv::StringRecord,
) -> Option<SourceRow> {
    if columns.len() < 2 {
        return None;
    }
    let tag = columns.get(1)?;
    if normalize_tag(tag) == "tag" {
        return None;
    }
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: translation_items(columns.get(0).unwrap_or_default(), source.allow_primary),
    })
}

fn parse_github_csv_row(source: &ManifestSource, columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() == 2 {
        if normalize_tag(columns.get(0)?) == "tag" {
            return None;
        }
        return Some(SourceRow {
            tag: columns.get(0)?.to_owned(),
            weight: None,
            translations: split_github_translations(columns.get(1)?, source),
        });
    }
    if columns.len() < 4 || columns.get(0)?.trim() == "danbooru_text" {
        return None;
    }
    let tag = columns.get(2)?;
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: split_github_translations(columns.get(3)?, source),
    })
}

fn parse_alias_csv_row(columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() < 4 {
        return None;
    }
    let tag = columns.get(0)?;
    if tag.trim() == "tag" {
        return None;
    }
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: split_translation_list(columns.get(3)?)
            .into_iter()
            .map(|text| TranslationRow {
                text,
                allow_primary: false,
            })
            .collect(),
    })
}

fn split_github_translations(
    raw_translations: &str,
    source: &ManifestSource,
) -> Vec<TranslationRow> {
    split_translation_list(raw_translations)
        .into_iter()
        .enumerate()
        .map(|(index, text)| TranslationRow {
            text,
            allow_primary: index == 0 && source.allow_primary,
        })
        .collect()
}

fn finalize_lexicon(
    records: HashMap<String, TagRecord>,
    manifest: &SourceManifest,
    ordering: CategoryOrdering,
) -> LexiconFile {
    let mut categorized = BTreeMap::<String, BTreeMap<String, Vec<TagRecord>>>::new();
    let mut uncategorized = BTreeMap::<String, Vec<TagRecord>>::new();
    for record in records.into_values() {
        if let (Some(category), Some(subcategory)) = (&record.category, &record.subcategory) {
            categorized
                .entry(category.clone())
                .or_default()
                .entry(subcategory.clone())
                .or_default()
                .push(record);
        } else {
            uncategorized
                .entry(classify_other_bucket(&record.tag))
                .or_default()
                .push(record);
        }
    }

    let mut output = LexiconOutputBuilder::new(manifest);
    for category in ordering.category_order {
        let Some(groups) = categorized.get_mut(&category) else {
            continue;
        };
        let subcategory_names = ordering
            .subcategory_order_by_category
            .get(&category)
            .cloned()
            .unwrap_or_else(|| groups.keys().cloned().collect());
        output.push_category(&category, groups, subcategory_names);
    }
    let other_buckets = uncategorized.keys().cloned().collect();
    output.push_category("other", &mut uncategorized, other_buckets);
    output.finish()
}

#[derive(Default)]
struct CategoryOrdering {
    category_order: Vec<String>,
    subcategory_order_by_category: BTreeMap<String, Vec<String>>,
}

struct CategorizedJsonSource {
    path: PathBuf,
    raw: serde_json::Value,
}

#[derive(Deserialize)]
struct CategoryOrderFile {
    categories: Vec<CategoryOrderCategory>,
}

#[derive(Deserialize)]
struct CategoryOrderCategory {
    name: String,
    subcategories: Vec<String>,
}

struct LexiconOutputBuilder {
    categories: Vec<LexiconCategory>,
    subcategories: Vec<LexiconSubcategory>,
    tags: Vec<LexiconTag>,
    translations: Vec<String>,
    stats: LexiconStats,
    sources: Vec<LexiconSource>,
}

impl LexiconOutputBuilder {
    fn new(manifest: &SourceManifest) -> Self {
        Self {
            categories: Vec::new(),
            subcategories: Vec::new(),
            tags: Vec::new(),
            translations: Vec::new(),
            stats: LexiconStats {
                source_count: manifest.sources.len() as u64,
                manifest_version: manifest.version,
                ..LexiconStats::default()
            },
            sources: manifest.sources.iter().map(LexiconSource::from).collect(),
        }
    }

    fn push_category(
        &mut self,
        name: &str,
        groups: &mut BTreeMap<String, Vec<TagRecord>>,
        subcategory_names: Vec<String>,
    ) {
        let subcategory_start = self.subcategories.len();
        let mut subcategory_count = 0usize;
        for subcategory in subcategory_names {
            let Some(records) = groups.get_mut(&subcategory) else {
                continue;
            };
            records.sort_by(compare_records_for_export);
            let tag_start = self.tags.len();
            for record in records {
                self.push_tag(record);
            }
            if self.tags.len() == tag_start {
                continue;
            }
            self.subcategories.push(LexiconSubcategory {
                name: subcategory,
                category_index: self.categories.len(),
                tag_start,
                tag_count: self.tags.len() - tag_start,
            });
            subcategory_count += 1;
        }
        if subcategory_count > 0 {
            self.categories.push(LexiconCategory {
                name: name.to_owned(),
                subcategory_start,
                subcategory_count,
            });
        }
    }

    fn push_tag(&mut self, record: &TagRecord) {
        let resolved = resolve_translations(record);
        let start = self.translations.len();
        self.translations.push(resolved.primary.clone());
        self.translations.extend(resolved.aliases.iter().cloned());
        self.tags.push(LexiconTag {
            tag: record.tag.clone(),
            weight: record.weight,
            translation_start: start,
            translation_count: 1 + resolved.aliases.len(),
        });
        self.stats.total_tags += 1;
        if record.category.is_some() {
            self.stats.categorized_tags += 1;
        } else {
            self.stats.uncategorized_tags += 1;
        }
        if record.weight.is_some() {
            self.stats.matched_weights += 1;
        }
        if !resolved.aliases.is_empty() {
            self.stats.tags_with_aliases += 1;
            self.stats.max_aliases_per_tag = self
                .stats
                .max_aliases_per_tag
                .max(resolved.aliases.len() as u64);
        }
        if let Some(source_id) = resolved.primary_source_id {
            if source_id.starts_with("categorized_json:") {
                self.stats.primary_from_category_json += 1;
            } else {
                self.stats.primary_from_manifest_sources += 1;
            }
        } else {
            self.stats.primary_fallback_to_tag += 1;
        }
    }

    fn finish(mut self) -> LexiconFile {
        self.stats.total_translations = self.translations.len() as u64;
        LexiconFile {
            schema: LEXICON_SCHEMA.to_owned(),
            version: LEXICON_VERSION,
            sources: self.sources,
            categories: self.categories,
            subcategories: self.subcategories,
            tags: self.tags,
            translations: self.translations,
            stats: self.stats,
        }
    }
}

fn load_source_manifest(path: &Path) -> Result<SourceManifest, String> {
    let raw_text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let raw: SourceManifestRaw =
        serde_json::from_str(&strip_bom(&raw_text)).map_err(|error| error.to_string())?;
    let manifest_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut sources = Vec::new();
    for (manifest_index, source) in raw.sources.iter().enumerate() {
        sources.push(ManifestSource::from_raw(
            source,
            manifest_dir,
            manifest_index,
        )?);
    }
    Ok(SourceManifest {
        version: raw.version.unwrap_or(1),
        sources,
    })
}

#[derive(Deserialize)]
struct SourceManifestRaw {
    version: Option<u32>,
    sources: Vec<ManifestSourceRaw>,
}

#[derive(Deserialize)]
struct ManifestSourceRaw {
    id: String,
    path: String,
    parser: String,
    priority: i64,
    #[serde(default)]
    alias_only: bool,
    allow_primary: Option<bool>,
}

#[derive(Clone)]
struct SourceManifest {
    version: u32,
    sources: Vec<ManifestSource>,
}

#[derive(Clone)]
struct ManifestSource {
    id: String,
    relative_path: String,
    path: PathBuf,
    parser: SourceParser,
    priority: i64,
    alias_only: bool,
    allow_primary: bool,
    manifest_index: usize,
}

impl ManifestSource {
    fn from_raw(
        raw: &ManifestSourceRaw,
        manifest_dir: &Path,
        manifest_index: usize,
    ) -> Result<Self, String> {
        let id = raw.id.trim().to_owned();
        let relative_path = raw.path.trim().to_owned();
        let parser = SourceParser::parse(raw.parser.trim())?;
        let allow_primary = raw.allow_primary.unwrap_or(!raw.alias_only);
        if id.is_empty() || relative_path.is_empty() {
            return Err("prompt lexicon source id and path must be non-empty".to_owned());
        }
        if raw.alias_only && allow_primary {
            return Err(format!(
                "prompt lexicon source `{id}` cannot be alias_only and allow_primary"
            ));
        }
        Ok(Self {
            id,
            path: manifest_dir.join(&relative_path),
            relative_path,
            parser,
            priority: raw.priority,
            alias_only: raw.alias_only,
            allow_primary,
            manifest_index,
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum SourceParser {
    Weighted,
    Simple,
    Reversed,
    Github,
    Alias,
}

impl SourceParser {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "weighted_csv" => Ok(Self::Weighted),
            "simple_csv" => Ok(Self::Simple),
            "reversed_csv" => Ok(Self::Reversed),
            "github_csv" => Ok(Self::Github),
            "alias_csv" => Ok(Self::Alias),
            _ => Err(format!("unsupported prompt lexicon parser `{value}`")),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Weighted => "weighted_csv",
            Self::Simple => "simple_csv",
            Self::Reversed => "reversed_csv",
            Self::Github => "github_csv",
            Self::Alias => "alias_csv",
        }
    }
}

#[derive(Clone)]
struct SourceRow {
    tag: String,
    weight: Option<u64>,
    translations: Vec<TranslationRow>,
}

#[derive(Clone)]
struct TranslationRow {
    text: String,
    allow_primary: bool,
}

#[derive(Clone)]
struct TagRecord {
    tag: String,
    normalized_tag: String,
    weight: Option<u64>,
    category: Option<String>,
    subcategory: Option<String>,
    translations: HashMap<String, TranslationCandidate>,
    next_translation_order: usize,
}

#[derive(Clone)]
struct TranslationCandidate {
    text: String,
    normalized: String,
    order: usize,
    primary_priority: Option<i64>,
    primary_source_id: Option<String>,
}

#[derive(Clone)]
struct TranslationInput {
    priority: i64,
    allow_primary: bool,
    source_id: String,
}

impl TranslationInput {
    const fn primary(priority: i64, source_id: String) -> Self {
        Self {
            priority,
            allow_primary: true,
            source_id,
        }
    }
}

#[derive(Clone)]
struct ResolvedTranslations {
    primary: String,
    aliases: Vec<String>,
    primary_source_id: Option<String>,
}

#[derive(Serialize)]
struct LexiconFile {
    schema: String,
    version: u32,
    sources: Vec<LexiconSource>,
    categories: Vec<LexiconCategory>,
    subcategories: Vec<LexiconSubcategory>,
    tags: Vec<LexiconTag>,
    translations: Vec<String>,
    stats: LexiconStats,
}

#[derive(Serialize)]
struct LexiconSource {
    id: String,
    path: String,
    parser: String,
    priority: i64,
    alias_only: bool,
    allow_primary: bool,
}

impl From<&ManifestSource> for LexiconSource {
    fn from(value: &ManifestSource) -> Self {
        Self {
            id: value.id.clone(),
            path: value.relative_path.clone(),
            parser: value.parser.as_str().to_owned(),
            priority: value.priority,
            alias_only: value.alias_only,
            allow_primary: value.allow_primary,
        }
    }
}

#[derive(Serialize)]
struct LexiconCategory {
    name: String,
    subcategory_start: usize,
    subcategory_count: usize,
}

#[derive(Serialize)]
struct LexiconSubcategory {
    name: String,
    category_index: usize,
    tag_start: usize,
    tag_count: usize,
}

#[derive(Serialize)]
struct LexiconTag {
    tag: String,
    weight: Option<u64>,
    translation_start: usize,
    translation_count: usize,
}

#[derive(Default, Serialize)]
struct LexiconStats {
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

fn ensure_tag_record<'a>(
    records: &'a mut HashMap<String, TagRecord>,
    raw_tag: &str,
) -> &'a mut TagRecord {
    let tag = normalize_tag_display(raw_tag);
    let normalized_tag = normalize_tag(&tag);
    records
        .entry(normalized_tag.clone())
        .or_insert_with(move || TagRecord {
            tag,
            normalized_tag,
            weight: None,
            category: None,
            subcategory: None,
            translations: HashMap::new(),
            next_translation_order: 0,
        })
}

fn merge_translation_candidate(
    record: &mut TagRecord,
    raw_translation: &str,
    input: TranslationInput,
) {
    let translation = normalize_display_text(raw_translation);
    let normalized = normalize_text(&translation);
    if translation.is_empty()
        || normalized.is_empty()
        || normalized == "none"
        || normalized == record.normalized_tag
    {
        return;
    }
    let candidate = record
        .translations
        .entry(normalized.clone())
        .or_insert_with(|| {
            let order = record.next_translation_order;
            record.next_translation_order += 1;
            TranslationCandidate {
                text: translation,
                normalized,
                order,
                primary_priority: None,
                primary_source_id: None,
            }
        });
    if input.allow_primary
        && candidate
            .primary_priority
            .is_none_or(|priority| input.priority > priority)
    {
        candidate.primary_priority = Some(input.priority);
        candidate.primary_source_id = Some(input.source_id);
    }
}

fn resolve_translations(record: &TagRecord) -> ResolvedTranslations {
    let primary_candidate = record
        .translations
        .values()
        .filter(|candidate| candidate.primary_priority.is_some())
        .max_by(|left, right| {
            left.primary_priority
                .cmp(&right.primary_priority)
                .then_with(|| right.order.cmp(&left.order))
        });
    let primary =
        primary_candidate.map_or_else(|| record.tag.clone(), |candidate| candidate.text.clone());
    let normalized_primary = normalize_text(&primary);
    let mut aliases = record
        .translations
        .values()
        .filter(|candidate| candidate.normalized != normalized_primary)
        .collect::<Vec<_>>();
    aliases.sort_by_key(|candidate| candidate.order);
    ResolvedTranslations {
        primary,
        aliases: aliases
            .into_iter()
            .map(|candidate| candidate.text.clone())
            .collect(),
        primary_source_id: primary_candidate
            .and_then(|candidate| candidate.primary_source_id.clone()),
    }
}

fn translation_items(value: &str, allow_primary: bool) -> Vec<TranslationRow> {
    let text = normalize_display_text(value);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![TranslationRow {
            text,
            allow_primary,
        }]
    }
}

fn split_translation_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(normalize_display_text)
        .filter(|value| !value.is_empty())
        .collect()
}

fn compare_manifest_source(left: &ManifestSource, right: &ManifestSource) -> Ordering {
    right
        .priority
        .cmp(&left.priority)
        .then_with(|| left.manifest_index.cmp(&right.manifest_index))
        .then_with(|| left.id.cmp(&right.id))
}

fn compare_records_for_export(left: &TagRecord, right: &TagRecord) -> Ordering {
    right
        .weight
        .unwrap_or(0)
        .cmp(&left.weight.unwrap_or(0))
        .then_with(|| left.tag.to_lowercase().cmp(&right.tag.to_lowercase()))
}

fn classify_other_bucket(tag: &str) -> String {
    let Some(first) = tag.trim().chars().next() else {
        return "#".to_owned();
    };
    let upper = first.to_ascii_uppercase();
    if upper.is_ascii_alphabetic() {
        upper.to_string()
    } else {
        "#".to_owned()
    }
}

fn is_valid_tag(tag: &str) -> bool {
    let trimmed = tag.trim();
    !normalize_tag(trimmed).is_empty() && trimmed.is_ascii()
}

fn normalize_tag_display(tag: &str) -> String {
    tag.trim().to_owned()
}

fn normalize_tag(tag: &str) -> String {
    normalize_comparable_text(&tag.replace('_', " "))
}

fn normalize_text(value: &str) -> String {
    normalize_comparable_text(value)
}

fn normalize_comparable_text(value: &str) -> String {
    value
        .nfkc()
        .collect::<String>()
        .trim()
        .to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_display_text(value: &str) -> String {
    value
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn read_json_value(path: &Path) -> Result<serde_json::Value, String> {
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&strip_bom(&raw)).map_err(|error| error.to_string())
}

fn strip_bom(value: &str) -> String {
    value.strip_prefix('\u{feff}').unwrap_or(value).to_owned()
}

fn unique_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
