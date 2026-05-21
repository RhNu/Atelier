use super::{
    BTreeMap, CATEGORY_PRIMARY_PRIORITY, Deserialize, HashMap, HashSet, LexiconFile, Path, PathBuf,
    PromptLexiconBuildConfig, SourceManifest, TagRecord, TranslationInput, compare_manifest_source,
    ensure_tag_record, finalize_lexicon, fs, ingest_source, is_valid_tag,
    merge_translation_candidate, read_json_value, strip_bom,
};

pub(super) fn merge_lexicon(
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

#[derive(Default)]
pub(super) struct CategoryOrdering {
    pub(super) category_order: Vec<String>,
    pub(super) subcategory_order_by_category: BTreeMap<String, Vec<String>>,
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
