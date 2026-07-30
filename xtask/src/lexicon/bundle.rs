use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use atelier_adapter_lexicon_bundle::{
    BUNDLE_FORMAT, BUNDLE_SCHEMA_VERSION, BundleFile, DATABASE_SCHEMA_VERSION, EnrichmentManifest,
    LexiconBundleManifest, RankingManifest, SemanticManifest, SemanticModelContract,
    SourceManifest, TokenizerEncoding, TokenizerFile,
};
use rusqlite::{Connection, params};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use super::input::{
    PipelineEnrichment, PipelineEntity, PipelineGroup, PipelineProvenance, PipelineRelation,
    SemanticConfig,
};
use super::schema::CREATE_SCHEMA;

#[derive(Clone, Debug)]
pub struct LexiconBundleConfig {
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
    pub bundle_version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconBundleSummary {
    pub output_dir: PathBuf,
    pub entity_count: usize,
    pub relation_count: usize,
    pub semantic_available: bool,
}

/// Builds a deterministic read-only runtime bundle from normalized pipeline output.
///
/// # Errors
/// Returns an error for malformed input, inconsistent relationships, or filesystem failures.
pub fn build_lexicon_bundle(config: &LexiconBundleConfig) -> Result<LexiconBundleSummary, String> {
    validate_bundle_version(&config.bundle_version)?;
    let base_entities_path = config.input_dir.join("entities.jsonl");
    let enriched_entities_path = config.input_dir.join("entities.enriched.jsonl");
    let entities_path = if enriched_entities_path.is_file() {
        enriched_entities_path
    } else {
        base_entities_path.clone()
    };
    let mut entities = read_jsonl::<PipelineEntity>(&entities_path)?;
    entities.sort_by_key(|entity| entity.id);
    validate_entities(&entities)?;
    let mut groups =
        read_json::<Vec<PipelineGroup>>(&config.input_dir.join("groups.json")).unwrap_or_default();
    groups.sort_by(|left, right| left.id.cmp(&right.id));
    let mut relations = read_jsonl::<PipelineRelation>(&config.input_dir.join("relations.jsonl"))
        .unwrap_or_default();
    relations.sort_by(|left, right| {
        (left.source_entity_id, left.target_entity_id, &left.relation).cmp(&(
            right.source_entity_id,
            right.target_entity_id,
            &right.relation,
        ))
    });
    validate_relations(&entities, &relations)?;

    fs::create_dir_all(&config.output_dir).map_err(|error| error.to_string())?;
    let database_path = config.output_dir.join("lexicon.sqlite");
    if database_path.exists() {
        fs::remove_file(&database_path).map_err(|error| error.to_string())?;
    }
    build_database(&database_path, &entities, &groups, &relations)?;
    let semantic = install_semantic_assets(
        config,
        entities.len(),
        &entities_path,
        entities_path != base_entities_path,
    )?;
    let enrichment = load_enrichment(config, &entities_path, entities.len())?;
    let provenance = read_json::<PipelineProvenance>(&config.input_dir.join("provenance.json"))
        .unwrap_or(PipelineProvenance { sources: vec![] });
    let manifest = LexiconBundleManifest {
        format: BUNDLE_FORMAT.to_owned(),
        schema_version: BUNDLE_SCHEMA_VERSION,
        bundle_version: config.bundle_version.clone(),
        database: describe_file(&database_path, "lexicon.sqlite")?,
        semantic,
        enrichment,
        ranking: RankingManifest::default(),
        sources: provenance
            .sources
            .into_iter()
            .map(|source| SourceManifest {
                id: source.id,
                url: source.url,
                snapshot: source.snapshot,
                sha256: source.sha256,
                license: source.license,
            })
            .collect(),
    };
    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())? + "\n";
    fs::write(config.output_dir.join("manifest.json"), manifest_json)
        .map_err(|error| error.to_string())?;
    Ok(LexiconBundleSummary {
        output_dir: config.output_dir.clone(),
        entity_count: entities.len(),
        relation_count: relations.len(),
        semantic_available: manifest.semantic.is_some(),
    })
}

fn build_database(
    path: &Path,
    entities: &[PipelineEntity],
    groups: &[PipelineGroup],
    relations: &[PipelineRelation],
) -> Result<(), String> {
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .execute_batch(CREATE_SCHEMA)
        .map_err(|error| error.to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO metadata(key, value) VALUES
             ('format', ?1), ('schema_version', ?2)",
            params![BUNDLE_FORMAT, DATABASE_SCHEMA_VERSION.to_string()],
        )
        .map_err(|error| error.to_string())?;

    let groups_by_entity = insert_groups(&transaction, groups)?;
    for (row_index, entity) in entities.iter().enumerate() {
        insert_entity(&transaction, entity, row_index, &groups_by_entity)?;
    }
    for relation in relations {
        transaction
            .execute(
                "INSERT INTO related_entities(
                    source_entity_id, target_entity_id, relation, npmi
                 ) VALUES (?1, ?2, ?3, ?4)",
                params![
                    sql_u64(relation.source_entity_id)?,
                    sql_u64(relation.target_entity_id)?,
                    relation.relation,
                    relation.npmi
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    connection
        .execute_batch("INSERT INTO entity_fts(entity_fts) VALUES('optimize'); VACUUM;")
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn insert_groups(
    connection: &Connection,
    groups: &[PipelineGroup],
) -> Result<BTreeMap<u64, Vec<(String, String)>>, String> {
    let mut by_entity = BTreeMap::<u64, Vec<(String, String)>>::new();
    for group in groups {
        let mut members = group.members.clone();
        members.sort_unstable();
        members.dedup();
        connection
            .execute(
                "INSERT INTO tag_groups(id, name, member_count) VALUES (?1, ?2, ?3)",
                params![group.id, group.name, sql_usize(members.len())?],
            )
            .map_err(|error| error.to_string())?;
        for entity_id in members {
            by_entity
                .entry(entity_id)
                .or_default()
                .push((group.id.clone(), group.name.clone()));
        }
    }
    Ok(by_entity)
}

fn insert_entity(
    connection: &Connection,
    entity: &PipelineEntity,
    row_index: usize,
    groups_by_entity: &BTreeMap<u64, Vec<(String, String)>>,
) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO entities(
                id, canonical_name, normalized_name, primary_translation,
                kind, category, post_count, rating
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                sql_u64(entity.id)?,
                entity.canonical_name,
                normalize_tag(&entity.canonical_name),
                entity.primary_translation,
                entity.kind,
                entity.category,
                sql_u64(entity.post_count)?,
                entity.rating
            ],
        )
        .map_err(|error| error.to_string())?;
    for alias in sorted_unique(&entity.aliases) {
        connection
            .execute(
                "INSERT INTO aliases(entity_id, alias, normalized_alias) VALUES (?1, ?2, ?3)",
                params![sql_u64(entity.id)?, alias, normalize_tag(alias)],
            )
            .map_err(|error| error.to_string())?;
    }
    let mut translation_text = Vec::new();
    for translation in &entity.translations {
        connection
            .execute(
                "INSERT INTO translations(entity_id, locale, text, normalized_text)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    sql_u64(entity.id)?,
                    translation.locale,
                    translation.text,
                    normalize_text(&translation.text)
                ],
            )
            .map_err(|error| error.to_string())?;
        translation_text.push(translation.text.as_str());
    }
    let mut wiki_text = Vec::new();
    for wiki in &entity.wiki {
        connection
            .execute(
                "INSERT INTO wiki(entity_id, locale, text) VALUES (?1, ?2, ?3)",
                params![sql_u64(entity.id)?, wiki.locale, wiki.text],
            )
            .map_err(|error| error.to_string())?;
        wiki_text.push(wiki.text.as_str());
    }
    let groups = groups_by_entity
        .get(&entity.id)
        .cloned()
        .unwrap_or_default();
    for (group_id, _) in &groups {
        connection
            .execute(
                "INSERT INTO tag_group_members(group_id, entity_id) VALUES (?1, ?2)",
                params![group_id, sql_u64(entity.id)?],
            )
            .map_err(|error| error.to_string())?;
    }
    connection
        .execute(
            "INSERT INTO semantic_rows(row_index, entity_id) VALUES (?1, ?2)",
            params![sql_usize(row_index)?, sql_u64(entity.id)?],
        )
        .map_err(|error| error.to_string())?;
    let aliases = sorted_unique(&entity.aliases)
        .into_iter()
        .chain(sorted_unique(&entity.expansion_terms))
        .collect::<Vec<_>>()
        .join(" ");
    let group_text = groups
        .iter()
        .map(|(_, name)| name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    connection
        .execute(
            "INSERT INTO entity_fts(
                entity_id, canonical_name, aliases, translations, wiki, groups_text
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                sql_u64(entity.id)?,
                entity.canonical_name,
                aliases,
                translation_text.join(" "),
                wiki_text.join(" "),
                group_text
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn install_semantic_assets(
    config: &LexiconBundleConfig,
    entity_count: usize,
    entities_path: &Path,
    enriched: bool,
) -> Result<Option<SemanticManifest>, String> {
    let source = config.input_dir.join("semantic");
    let config_path = source.join("config.json");
    if !config_path.is_file() {
        return Ok(None);
    }
    let semantic = read_json::<SemanticConfig>(&config_path)?;
    if semantic.entity_count != entity_count {
        return Err(format!(
            "semantic entity_count {} does not match pipeline entities {entity_count}",
            semantic.entity_count
        ));
    }
    let entities_sha256 = sha256_file(entities_path)?;
    match &semantic.entities_sha256 {
        Some(expected) if expected != &entities_sha256 => {
            return Err(
                "semantic vectors were built from a different entities file; rebuild them"
                    .to_owned(),
            );
        }
        None if enriched => {
            return Err(
                "semantic config does not bind vectors to enriched entities; rebuild them"
                    .to_owned(),
            );
        }
        _ => {}
    }
    let files = [
        ("model.onnx", "model.onnx"),
        ("LICENSE-model.txt", "LICENSE-model.txt"),
        ("identity.f16", "identity.f16"),
        ("knowledge.f16", "knowledge.f16"),
    ];
    for (source_name, output_name) in files {
        fs::copy(
            source.join(source_name),
            config.output_dir.join(output_name),
        )
        .map_err(|error| format!("failed to install semantic asset {source_name}: {error}"))?;
    }
    let tokenizer = install_tokenizer(
        &source.join("tokenizer.json"),
        &config.output_dir.join("tokenizer.json.zst"),
    )?;
    let stale_tokenizer = config.output_dir.join("tokenizer.json");
    if stale_tokenizer.is_file() {
        fs::remove_file(&stale_tokenizer)
            .map_err(|error| format!("failed to remove stale tokenizer.json: {error}"))?;
    }
    Ok(Some(SemanticManifest {
        model: describe_file(&config.output_dir.join("model.onnx"), "model.onnx")?,
        tokenizer,
        license: describe_file(
            &config.output_dir.join("LICENSE-model.txt"),
            "LICENSE-model.txt",
        )?,
        identity_vectors: describe_file(&config.output_dir.join("identity.f16"), "identity.f16")?,
        knowledge_vectors: describe_file(
            &config.output_dir.join("knowledge.f16"),
            "knowledge.f16",
        )?,
        dimensions: semantic.dimensions,
        entity_count,
        max_length: semantic.max_length,
        model_contract: SemanticModelContract {
            input_ids: semantic.input_ids,
            attention_mask: semantic.attention_mask,
            token_type_ids: semantic.token_type_ids,
            output_name: semantic.output_name,
            pooling: "mean".to_owned(),
            normalize: true,
            query_prefix: semantic.query_prefix,
            passage_prefix: semantic.passage_prefix,
        },
    }))
}

fn install_tokenizer(source: &Path, output: &Path) -> Result<TokenizerFile, String> {
    let content = fs::read(source)
        .map_err(|error| format!("failed to read tokenizer {}: {error}", source.display()))?;
    let compressed = zstd::bulk::compress(&content, 19)
        .map_err(|error| format!("failed to compress tokenizer: {error}"))?;
    fs::write(output, &compressed)
        .map_err(|error| format!("failed to install tokenizer {}: {error}", output.display()))?;
    Ok(TokenizerFile {
        bundle: describe_file(output, "tokenizer.json.zst")?,
        encoding: TokenizerEncoding::ZstdJson,
        content_sha256: format!("{:x}", Sha256::digest(&content)),
        content_size_bytes: content.len() as u64,
    })
}

fn load_enrichment(
    config: &LexiconBundleConfig,
    entities_path: &Path,
    entity_count: usize,
) -> Result<Option<EnrichmentManifest>, String> {
    if entities_path.file_name().and_then(|name| name.to_str()) != Some("entities.enriched.jsonl") {
        return Ok(None);
    }
    let path = config.input_dir.join("entities.enriched.provenance.json");
    let value = read_json::<PipelineEnrichment>(&path)?;
    if value.mode != "batch"
        || value.endpoint != "/v1/chat/completions"
        || value.entity_count != entity_count
        || value.input_sha256 != sha256_file(&config.input_dir.join("entities.jsonl"))?
        || value.output_sha256 != sha256_file(entities_path)?
    {
        return Err(format!(
            "{} does not match the selected enriched entities",
            path.display()
        ));
    }
    Ok(Some(EnrichmentManifest {
        mode: value.mode,
        endpoint: value.endpoint,
        model: value.model,
        prompt_hash: value.prompt_hash,
        entity_count: value.entity_count,
        input_sha256: value.input_sha256,
        output_sha256: value.output_sha256,
    }))
}

fn describe_file(path: &Path, relative: &str) -> Result<BundleFile, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(BundleFile {
        file: relative.to_owned(),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        size_bytes: bytes.len() as u64,
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_entities(entities: &[PipelineEntity]) -> Result<(), String> {
    if entities.is_empty() {
        return Err("pipeline contains no entities".to_owned());
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for entity in entities {
        if !ids.insert(entity.id) {
            return Err(format!("duplicate entity id {}", entity.id));
        }
        if !names.insert(normalize_tag(&entity.canonical_name)) {
            return Err(format!("duplicate canonical tag {}", entity.canonical_name));
        }
        if !matches!(entity.kind.as_str(), "tag" | "artist")
            || !matches!(
                entity.category.as_str(),
                "general" | "copyright" | "character" | "artist"
            )
            || !matches!(entity.rating.as_str(), "safe" | "sensitive" | "unknown")
        {
            return Err(format!(
                "entity {} has invalid kind/category/rating",
                entity.id
            ));
        }
    }
    Ok(())
}

fn validate_relations(
    entities: &[PipelineEntity],
    relations: &[PipelineRelation],
) -> Result<(), String> {
    let ids = entities
        .iter()
        .map(|entity| entity.id)
        .collect::<HashSet<_>>();
    for relation in relations {
        if !ids.contains(&relation.source_entity_id)
            || !ids.contains(&relation.target_entity_id)
            || !relation.npmi.is_finite()
            || !(0.0..=1.0).contains(&relation.npmi)
        {
            return Err(format!(
                "invalid relation {} -> {}",
                relation.source_entity_id, relation.target_entity_id
            ));
        }
    }
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn read_jsonl<T: DeserializeOwned>(path: &Path) -> Result<Vec<T>, String> {
    let file = fs::File::open(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            other => Some((index, other)),
        })
        .map(|(index, line)| {
            let line = line.map_err(|error| error.to_string())?;
            serde_json::from_str(&line)
                .map_err(|error| format!("{} line {}: {error}", path.display(), index + 1))
        })
        .collect()
}

fn sorted_unique(values: &[String]) -> Vec<&str> {
    let mut values = values.iter().map(String::as_str).collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

fn normalize_tag(value: &str) -> String {
    value
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_whitespace() {
                '_'
            } else {
                character
            }
        })
        .collect()
}

fn normalize_text(value: &str) -> String {
    value
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|character| if character == '_' { ' ' } else { character })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn validate_bundle_version(version: &str) -> Result<(), String> {
    if version.trim().is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
    {
        return Err("bundle version must be a non-empty safe identifier".to_owned());
    }
    Ok(())
}

fn sql_u64(value: u64) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("value {value} exceeds SQLite integer range"))
}

fn sql_usize(value: usize) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("value {value} exceeds SQLite integer range"))
}
