use super::{
    BTreeMap, CategoryOrdering, HashMap, LEXICON_SCHEMA, LEXICON_VERSION, ManifestSource,
    Serialize, SourceManifest, TagRecord, classify_other_bucket, compare_records_for_export,
    resolve_translations,
};

pub(super) fn finalize_lexicon(
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

#[derive(Serialize)]
pub(super) struct LexiconFile {
    pub(super) schema: String,
    pub(super) version: u32,
    pub(super) sources: Vec<LexiconSource>,
    pub(super) categories: Vec<LexiconCategory>,
    pub(super) subcategories: Vec<LexiconSubcategory>,
    pub(super) tags: Vec<LexiconTag>,
    pub(super) translations: Vec<String>,
    pub(super) stats: LexiconStats,
}

#[derive(Serialize)]
pub(super) struct LexiconSource {
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
pub(super) struct LexiconCategory {
    name: String,
    subcategory_start: usize,
    subcategory_count: usize,
}

#[derive(Serialize)]
pub(super) struct LexiconSubcategory {
    name: String,
    category_index: usize,
    tag_start: usize,
    tag_count: usize,
}

#[derive(Serialize)]
pub(super) struct LexiconTag {
    tag: String,
    weight: Option<u64>,
    translation_start: usize,
    translation_count: usize,
}

#[derive(Default, Serialize)]
pub(super) struct LexiconStats {
    pub(super) total_tags: u64,
    pub(super) categorized_tags: u64,
    pub(super) uncategorized_tags: u64,
    pub(super) matched_weights: u64,
    pub(super) total_translations: u64,
    pub(super) tags_with_aliases: u64,
    pub(super) max_aliases_per_tag: u64,
    pub(super) source_count: u64,
    pub(super) manifest_version: u32,
    pub(super) primary_from_category_json: u64,
    pub(super) primary_from_manifest_sources: u64,
    pub(super) primary_fallback_to_tag: u64,
}
