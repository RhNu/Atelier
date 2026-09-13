use std::collections::BTreeMap;

use atelier_prompt_resources::{PromptChunkKey, rewrite_chunk_references};
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::Value;

use crate::error::DatabaseResult;

const TO_VERSION: i64 = 6;

const CREATE_REPLACEMENT_TABLES: &str = r"
CREATE TABLE prompt_chunks_v6 (
    chunk_id TEXT PRIMARY KEY,
    chunk_key TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    category TEXT,
    description TEXT,
    preview_resource_id TEXT,
    preview_variant_id TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE TABLE prompt_chunk_models_v6 (
    chunk_id TEXT NOT NULL,
    model TEXT NOT NULL,
    PRIMARY KEY (chunk_id, model),
    FOREIGN KEY (chunk_id) REFERENCES prompt_chunks_v6(chunk_id) ON DELETE CASCADE
);
CREATE TABLE prompt_presets_v6 (
    preset_id TEXT PRIMARY KEY,
    preset_kind TEXT NOT NULL,
    name TEXT NOT NULL,
    category TEXT,
    description TEXT,
    sort_order INTEGER NOT NULL,
    prompt_mode TEXT NOT NULL CHECK (prompt_mode IN ('surround', 'replace')),
    uc_mode TEXT NOT NULL CHECK (uc_mode IN ('surround', 'replace')),
    before_text TEXT NOT NULL,
    after_text TEXT NOT NULL,
    replace_text TEXT NOT NULL,
    uc_before_text TEXT NOT NULL,
    uc_after_text TEXT NOT NULL,
    uc_replace_text TEXT NOT NULL,
    quality_override TEXT,
    uc_preset_override TEXT,
    preview_resource_id TEXT,
    preview_variant_id TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE TABLE prompt_preset_models_v6 (
    preset_id TEXT NOT NULL,
    model TEXT NOT NULL,
    PRIMARY KEY (preset_id, model),
    FOREIGN KEY (preset_id) REFERENCES prompt_presets_v6(preset_id) ON DELETE CASCADE
);
CREATE TABLE vibe_documents_v6 (
    vibe_id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    has_image INTEGER NOT NULL,
    document_json TEXT NOT NULL
);
CREATE TABLE vibe_encodings_v6 (
    cache_key TEXT PRIMARY KEY,
    vibe_id TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    information_extracted_key TEXT NOT NULL,
    record_json TEXT NOT NULL
);
";

pub(super) fn migrate(connection: &mut rusqlite::Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    let id_maps = id_maps(&transaction)?;
    let chunk_paths = chunk_path_maps(&transaction)?;
    transaction.execute_batch(CREATE_REPLACEMENT_TABLES)?;
    copy_prompt_chunks(&transaction, &id_maps, &chunk_paths)?;
    copy_prompt_presets(&transaction, &id_maps, &chunk_paths)?;
    copy_vibes(&transaction, &id_maps)?;
    update_resource_owners(&transaction, &id_maps)?;
    migrate_generation_draft(&transaction, &id_maps, &chunk_paths)?;
    replace_tables(&transaction)?;
    transaction.execute(
        "UPDATE resource_library_nodes SET owner_local_id = node_id WHERE node_kind = 'resource'",
        [],
    )?;
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = ?1 WHERE singleton = 1",
        [TO_VERSION],
    )?;
    transaction.commit()?;
    Ok(TO_VERSION)
}

fn id_maps(transaction: &Transaction<'_>) -> DatabaseResult<BTreeMap<String, String>> {
    let mut statement = transaction.prepare(
        "SELECT owner_local_id, node_id FROM resource_library_nodes WHERE node_kind = 'resource' ORDER BY node_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn chunk_path_maps(transaction: &Transaction<'_>) -> DatabaseResult<BTreeMap<String, String>> {
    let mut statement = transaction.prepare(
        r"
        WITH RECURSIVE paths(node_id, path) AS (
            SELECT node_id, identifier FROM resource_library_nodes
            WHERE namespace = 'prompt_chunk' AND parent_folder_id IS NULL
            UNION ALL
            SELECT child.node_id, paths.path || '/' || child.identifier
            FROM resource_library_nodes child JOIN paths ON child.parent_folder_id = paths.node_id
            WHERE child.namespace = 'prompt_chunk'
        )
        SELECT chunks.chunk_key, paths.path
        FROM prompt_chunks chunks
        JOIN resource_library_nodes nodes ON nodes.owner_local_id = chunks.chunk_id
        JOIN paths ON paths.node_id = nodes.node_id
        ",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn copy_prompt_chunks(
    transaction: &Transaction<'_>,
    ids: &BTreeMap<String, String>,
    paths: &BTreeMap<String, String>,
) -> DatabaseResult<()> {
    let mut statement = transaction.prepare(
        "SELECT chunk_id, chunk_key, content, description, preview_resource_id, preview_variant_id, created_at_ms, updated_at_ms FROM prompt_chunks ORDER BY chunk_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
        ))
    })?;
    for row in rows {
        let (old_id, old_key, content, description, preview, variant, created, updated) = row?;
        let new_id = &ids[&old_id];
        let new_key = &paths[&old_key];
        let content = rewrite_text(&content, paths);
        transaction.execute(
            "INSERT INTO prompt_chunks_v6 VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id,
                new_key,
                content,
                description,
                preview,
                variant,
                created,
                updated
            ],
        )?;
        transaction.execute(
            "INSERT INTO prompt_chunk_models_v6 SELECT ?1, model FROM prompt_chunk_models WHERE chunk_id = ?2",
            params![new_id, old_id],
        )?;
    }
    Ok(())
}

fn copy_prompt_presets(
    transaction: &Transaction<'_>,
    ids: &BTreeMap<String, String>,
    chunk_paths: &BTreeMap<String, String>,
) -> DatabaseResult<()> {
    let mut statement = transaction.prepare(
        r"
        WITH RECURSIVE paths(node_id, path) AS (
            SELECT node_id, identifier FROM resource_library_nodes WHERE parent_folder_id IS NULL
            UNION ALL SELECT child.node_id, paths.path || '/' || child.identifier
            FROM resource_library_nodes child JOIN paths ON child.parent_folder_id = paths.node_id
        )
        SELECT p.preset_id, p.preset_kind, n.display_name, paths.path, p.description,
               p.prompt_mode, p.uc_mode, p.before_text, p.after_text, p.replace_text,
               p.uc_before_text, p.uc_after_text, p.uc_replace_text, p.quality_override,
               p.uc_preset_override, p.preview_resource_id, p.preview_variant_id,
               p.created_at_ms, p.updated_at_ms
        FROM prompt_presets p
        JOIN resource_library_nodes n ON n.owner_local_id = p.preset_id
        JOIN paths ON paths.node_id = n.node_id
        ORDER BY p.preset_id
        ",
    )?;
    let rows = statement.query_map([], |row| {
        (0..19)
            .map(|index| row.get::<_, ValueCell>(index))
            .collect::<Result<Vec<_>, _>>()
    })?;
    for row in rows {
        let values = row?;
        let old_id = values[0].text().to_owned();
        let new_id = ids[&old_id].clone();
        let mut text_values = values;
        for index in [7_usize, 8, 9, 10, 11, 12] {
            if let ValueCell::Text(text) = &mut text_values[index] {
                *text = rewrite_text(text, chunk_paths);
            }
        }
        transaction.execute(
            r"INSERT INTO prompt_presets_v6 VALUES (
                ?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                ?14, ?15, ?16, ?17, ?18, ?19
            )",
            rusqlite::params_from_iter(
                std::iter::once(ValueCell::Text(new_id.clone()))
                    .chain(text_values.into_iter().skip(1))
                    .map(ValueCell::into_value),
            ),
        )?;
        transaction.execute(
            "INSERT INTO prompt_preset_models_v6 SELECT ?1, model FROM prompt_preset_models WHERE preset_id = ?2",
            params![new_id, old_id],
        )?;
    }
    Ok(())
}

#[derive(Clone, Debug)]
enum ValueCell {
    Null,
    Integer(i64),
    Text(String),
}

impl ValueCell {
    fn text(&self) -> &str {
        match self {
            Self::Text(value) => value,
            Self::Null | Self::Integer(_) => panic!("expected migration text cell"),
        }
    }

    fn into_value(self) -> rusqlite::types::Value {
        match self {
            Self::Null => rusqlite::types::Value::Null,
            Self::Integer(value) => rusqlite::types::Value::Integer(value),
            Self::Text(value) => rusqlite::types::Value::Text(value),
        }
    }
}

impl rusqlite::types::FromSql for ValueCell {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(match value {
            rusqlite::types::ValueRef::Null => Self::Null,
            rusqlite::types::ValueRef::Integer(value) => Self::Integer(value),
            rusqlite::types::ValueRef::Text(value) => {
                Self::Text(String::from_utf8_lossy(value).into_owned())
            }
            _ => return Err(rusqlite::types::FromSqlError::InvalidType),
        })
    }
}

fn copy_vibes(transaction: &Transaction<'_>, ids: &BTreeMap<String, String>) -> DatabaseResult<()> {
    let mut statement = transaction.prepare(
        "SELECT vibe_id, display_name, has_image, document_json FROM vibe_documents ORDER BY vibe_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (source_id, display_name, has_image, json) = row?;
        let new_id = &ids[&source_id];
        let mut value: Value = serde_json::from_str(&json)?;
        if let Some(summary) = value.get_mut("summary").and_then(Value::as_object_mut) {
            summary.insert("document_id".to_owned(), Value::String(new_id.clone()));
            summary.insert("source_id".to_owned(), Value::String(source_id.clone()));
        }
        transaction.execute(
            "INSERT INTO vibe_documents_v6 VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                new_id,
                source_id,
                display_name,
                has_image,
                serde_json::to_string(&value)?
            ],
        )?;
        let mut encodings = transaction.prepare(
            "SELECT cache_key, source_hash, model, information_extracted_key, record_json FROM vibe_encodings WHERE vibe_id = ?1",
        )?;
        let encoding_rows = encodings.query_map([&source_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for encoding in encoding_rows {
            let (cache_key, source_hash, model, info, json) = encoding?;
            let mut value: Value = serde_json::from_str(&json)?;
            if let Some(object) = value.as_object_mut() {
                object.insert("vibe_id".to_owned(), Value::String(new_id.clone()));
            }
            transaction.execute(
                "INSERT INTO vibe_encodings_v6 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    cache_key,
                    new_id,
                    source_hash,
                    model,
                    info,
                    serde_json::to_string(&value)?
                ],
            )?;
        }
    }
    Ok(())
}

fn update_resource_owners(
    transaction: &Transaction<'_>,
    ids: &BTreeMap<String, String>,
) -> DatabaseResult<()> {
    for (old, new) in ids {
        for prefix in ["chunk:", "preset:"] {
            transaction.execute(
                "UPDATE resource_links SET owner_local_id = ?2 WHERE owner_kind = 'prompt_resource' AND owner_local_id = ?1",
                params![format!("{prefix}{old}"), format!("{prefix}{new}")],
            )?;
        }
        transaction.execute(
            "UPDATE resource_links SET owner_local_id = ?2 WHERE owner_kind = 'vibe' AND owner_local_id = ?1",
            params![old, new],
        )?;
    }
    Ok(())
}

fn migrate_generation_draft(
    transaction: &Transaction<'_>,
    ids: &BTreeMap<String, String>,
    paths: &BTreeMap<String, String>,
) -> DatabaseResult<()> {
    let json = transaction
        .query_row(
            "SELECT value_json FROM workspace_settings WHERE setting_key = 'generation.draft'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(json) = json else {
        return Ok(());
    };
    let mut value: Value = serde_json::from_str(&json)?;
    rewrite_json(&mut value, ids, paths);
    transaction.execute(
        "UPDATE workspace_settings SET value_json = ?1 WHERE setting_key = 'generation.draft'",
        [serde_json::to_string(&value)?],
    )?;
    Ok(())
}

fn rewrite_json(
    value: &mut Value,
    ids: &BTreeMap<String, String>,
    paths: &BTreeMap<String, String>,
) {
    match value {
        Value::String(text) => {
            if let Some(replacement) = ids.get(text) {
                *text = replacement.clone();
            } else {
                *text = rewrite_text(text, paths);
            }
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| rewrite_json(value, ids, paths)),
        Value::Object(values) => values
            .values_mut()
            .for_each(|value| rewrite_json(value, ids, paths)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn rewrite_text(text: &str, paths: &BTreeMap<String, String>) -> String {
    paths.iter().fold(text.to_owned(), |current, (old, new)| {
        match (PromptChunkKey::parse(old), PromptChunkKey::parse(new)) {
            (Ok(old), Ok(new)) => rewrite_chunk_references(&current, &old, &new),
            _ => current,
        }
    })
}

fn replace_tables(transaction: &Transaction<'_>) -> DatabaseResult<()> {
    transaction.execute_batch(
        r"
        DROP TABLE prompt_chunk_models;
        DROP TABLE prompt_chunks;
        DROP TABLE prompt_preset_models;
        DROP TABLE prompt_presets;
        DROP TABLE vibe_encodings;
        DROP TABLE vibe_documents;
        ALTER TABLE prompt_chunks_v6 RENAME TO prompt_chunks;
        ALTER TABLE prompt_chunk_models_v6 RENAME TO prompt_chunk_models;
        ALTER TABLE prompt_presets_v6 RENAME TO prompt_presets;
        ALTER TABLE prompt_preset_models_v6 RENAME TO prompt_preset_models;
        ALTER TABLE vibe_documents_v6 RENAME TO vibe_documents;
        ALTER TABLE vibe_encodings_v6 RENAME TO vibe_encodings;
        CREATE INDEX idx_prompt_chunks_key ON prompt_chunks(chunk_key);
        CREATE INDEX idx_prompt_chunk_models_model ON prompt_chunk_models(model, chunk_id);
        CREATE INDEX idx_prompt_presets_kind_order ON prompt_presets(preset_kind, category, preset_id);
        CREATE INDEX idx_prompt_preset_models_model ON prompt_preset_models(model, preset_id);
        CREATE INDEX idx_vibe_encodings_lookup ON vibe_encodings(source_hash, model, information_extracted_key);
        ",
    )?;
    Ok(())
}
