#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use atelier_prompt_resources::{
    ChunkReference, PromptChunk, PromptChunkId, PromptChunkKey, PromptPreset, PromptPresetBehavior,
    PromptPresetId, PromptPresetKind, PromptResourceError, PromptResourceReader,
    PromptResourceRepository, PromptResourceResult, rewrite_chunk_references,
};
use atelier_resource_catalog::{ResourceId, ResourceRef, VariantId};
use rusqlite::{OptionalExtension, Params, params};

use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;

#[derive(Clone, Debug)]
pub struct DatabasePromptResourceRepository {
    connection: DatabaseConnection,
}

impl DatabasePromptResourceRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl PromptResourceReader for DatabasePromptResourceRepository {
    async fn get_chunk_by_id(
        &self,
        id: &PromptChunkId,
    ) -> PromptResourceResult<Option<PromptChunk>> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        connection
            .query_row(
                prompt_chunk_select("WHERE chunk_id = ?1").as_str(),
                params![id.as_str()],
                prompt_chunk_from_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn get_chunk_by_key(
        &self,
        key: &PromptChunkKey,
    ) -> PromptResourceResult<Option<PromptChunk>> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        connection
            .query_row(
                prompt_chunk_select("WHERE chunk_key = ?1").as_str(),
                params![key.as_str()],
                prompt_chunk_from_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn list_chunks(&self) -> PromptResourceResult<Vec<PromptChunk>> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        let mut statement = connection
            .prepare(prompt_chunk_select("ORDER BY chunk_key ASC").as_str())
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], prompt_chunk_from_row)
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }

    async fn get_preset_by_id(
        &self,
        id: &PromptPresetId,
    ) -> PromptResourceResult<Option<PromptPreset>> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        connection
            .query_row(
                prompt_preset_select("WHERE preset_id = ?1").as_str(),
                params![id.as_str()],
                prompt_preset_from_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn list_presets(
        &self,
        kind: Option<PromptPresetKind>,
    ) -> PromptResourceResult<Vec<PromptPreset>> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        let (where_clause, kind_text) = kind.map_or(("", None), |kind| {
            ("WHERE preset_kind = ?1", Some(preset_kind_to_str(kind)))
        });
        let mut statement = connection
            .prepare(
                prompt_preset_select(&format!(
                    "{where_clause} ORDER BY sort_order ASC, name ASC, preset_id ASC"
                ))
                .as_str(),
            )
            .map_err(sql_error)?;
        let rows = if let Some(kind_text) = kind_text {
            statement
                .query_map(params![kind_text], prompt_preset_from_row)
                .map_err(sql_error)?
        } else {
            statement
                .query_map([], prompt_preset_from_row)
                .map_err(sql_error)?
        };
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }
}

#[async_trait]
impl PromptResourceRepository for DatabasePromptResourceRepository {
    async fn allocate_chunk_id(&self) -> PromptResourceResult<PromptChunkId> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        let mut next = connection
            .query_row("SELECT COUNT(*) + 1 FROM prompt_chunks", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(sql_error)?
            .max(1);
        loop {
            let id = PromptChunkId::new(format!("chunk-{next}"));
            let exists = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM prompt_chunks WHERE chunk_id = ?1)",
                    params![id.as_str()],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(sql_error)?;
            if !exists {
                return Ok(id);
            }
            next += 1;
        }
    }

    async fn save_chunk(&self, chunk: PromptChunk) -> PromptResourceResult<()> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        upsert_chunk(&*connection, &chunk)
    }

    async fn save_chunk_and_rewrite_references(
        &self,
        chunk: PromptChunk,
        old_key: &PromptChunkKey,
    ) -> PromptResourceResult<()> {
        let mut connection = self.connection.lock().map_err(prompt_error)?;
        let tx = connection.transaction().map_err(sql_error)?;
        upsert_chunk(&tx, &chunk)?;

        let mut statement = tx
            .prepare(
                "SELECT chunk_id, content FROM prompt_chunks WHERE chunk_id <> ?1 ORDER BY chunk_id",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(params![chunk.id.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(sql_error)?;
        let items = rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?;
        drop(statement);

        for (chunk_id, content) in items {
            let rewritten = rewrite_chunk_references(&content, old_key, &chunk.key);
            if rewritten != content {
                tx.execute(
                    "UPDATE prompt_chunks SET content = ?2, updated_at_ms = ?3 WHERE chunk_id = ?1",
                    params![
                        chunk_id,
                        rewritten,
                        i64::try_from(chunk.updated_at_ms)
                            .map_err(|error| PromptResourceError::repository(error.to_string()))?,
                    ],
                )
                .map_err(sql_error)?;
            }
        }

        for (preset_id, column, content) in prompt_preset_text_fields(&tx)? {
            let rewritten = rewrite_chunk_references(&content, old_key, &chunk.key);
            if rewritten != content {
                let sql = format!(
                    "UPDATE prompt_presets SET {column} = ?2, updated_at_ms = ?3 WHERE preset_id = ?1"
                );
                tx.execute(
                    &sql,
                    params![
                        preset_id,
                        rewritten,
                        i64::try_from(chunk.updated_at_ms)
                            .map_err(|error| PromptResourceError::repository(error.to_string()))?,
                    ],
                )
                .map_err(sql_error)?;
            }
        }
        tx.commit().map_err(sql_error)
    }

    async fn delete_chunk(&self, id: &PromptChunkId) -> PromptResourceResult<()> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        connection
            .execute(
                "DELETE FROM prompt_chunks WHERE chunk_id = ?1",
                params![id.as_str()],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn list_chunk_references(
        &self,
        key: &PromptChunkKey,
    ) -> PromptResourceResult<Vec<ChunkReference>> {
        let mut references = self
            .list_chunks()
            .await?
            .into_iter()
            .filter(|chunk| chunk.references_chunk(key))
            .map(|chunk| ChunkReference {
                chunk_id: chunk.id,
                key: chunk.key,
            })
            .collect::<Vec<_>>();
        references.extend(
            self.list_presets(None)
                .await?
                .into_iter()
                .filter(|preset| preset.references_chunk(key))
                .map(|preset| ChunkReference {
                    chunk_id: PromptChunkId::new(preset.id.as_str()),
                    key: key.clone(),
                }),
        );
        Ok(references)
    }

    async fn allocate_preset_id(&self) -> PromptResourceResult<PromptPresetId> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        let mut next = connection
            .query_row("SELECT COUNT(*) + 1 FROM prompt_presets", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(sql_error)?
            .max(1);
        loop {
            let id = PromptPresetId::new(format!("preset-{next}"));
            let exists = connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM prompt_presets WHERE preset_id = ?1)",
                    params![id.as_str()],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(sql_error)?;
            if !exists {
                return Ok(id);
            }
            next += 1;
        }
    }

    async fn save_preset(&self, preset: PromptPreset) -> PromptResourceResult<()> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        upsert_preset(&*connection, &preset)
    }

    async fn delete_preset(&self, id: &PromptPresetId) -> PromptResourceResult<()> {
        let connection = self.connection.lock().map_err(prompt_error)?;
        connection
            .execute(
                "DELETE FROM prompt_presets WHERE preset_id = ?1",
                params![id.as_str()],
            )
            .map(|_| ())
            .map_err(sql_error)
    }
}

fn upsert_chunk(connection: &impl SqlExecutor, chunk: &PromptChunk) -> PromptResourceResult<()> {
    connection
        .execute_sql(
            r"
            INSERT INTO prompt_chunks(
                chunk_id, chunk_key, content, category, description,
                preview_resource_id, preview_variant_id, created_at_ms, updated_at_ms
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(chunk_id) DO UPDATE SET
                chunk_key = excluded.chunk_key,
                content = excluded.content,
                category = excluded.category,
                description = excluded.description,
                preview_resource_id = excluded.preview_resource_id,
                preview_variant_id = excluded.preview_variant_id,
                updated_at_ms = excluded.updated_at_ms
            ",
            params![
                chunk.id.as_str(),
                chunk.key.as_str(),
                chunk.content,
                chunk.category.as_deref(),
                chunk.description.as_deref(),
                chunk.preview_thumb.as_ref().map(|value| value.id.as_str()),
                chunk
                    .preview_thumb
                    .as_ref()
                    .and_then(|value| value.variant_id.as_ref())
                    .map(VariantId::as_str),
                i64::try_from(chunk.created_at_ms)
                    .map_err(|error| PromptResourceError::repository(error.to_string()))?,
                i64::try_from(chunk.updated_at_ms)
                    .map_err(|error| PromptResourceError::repository(error.to_string()))?,
            ],
        )
        .map(|_| ())
        .map_err(sql_error)
}

fn upsert_preset(connection: &impl SqlExecutor, preset: &PromptPreset) -> PromptResourceResult<()> {
    let (prompt_mode, before, after, replace) = preset_behavior_fields(&preset.prompt_behavior);
    let (uc_mode, uc_before, uc_after, uc_replace) = preset_behavior_fields(&preset.uc_behavior);
    connection
        .execute_sql(
            r"
            INSERT INTO prompt_presets(
                preset_id, preset_kind, name, category, description, sort_order, enabled,
                prompt_mode, uc_mode, before_text, after_text, replace_text, uc_before_text,
                uc_after_text, uc_replace_text, quality_override, uc_preset_override,
                preview_resource_id, preview_variant_id, created_at_ms, updated_at_ms
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            ON CONFLICT(preset_id) DO UPDATE SET
                preset_kind = excluded.preset_kind,
                name = excluded.name,
                category = excluded.category,
                description = excluded.description,
                sort_order = excluded.sort_order,
                enabled = excluded.enabled,
                prompt_mode = excluded.prompt_mode,
                uc_mode = excluded.uc_mode,
                before_text = excluded.before_text,
                after_text = excluded.after_text,
                replace_text = excluded.replace_text,
                uc_before_text = excluded.uc_before_text,
                uc_after_text = excluded.uc_after_text,
                uc_replace_text = excluded.uc_replace_text,
                quality_override = excluded.quality_override,
                uc_preset_override = excluded.uc_preset_override,
                preview_resource_id = excluded.preview_resource_id,
                preview_variant_id = excluded.preview_variant_id,
                updated_at_ms = excluded.updated_at_ms
            ",
            params![
                preset.id.as_str(),
                preset_kind_to_str(preset.kind),
                preset.name,
                preset.category.as_deref(),
                preset.description.as_deref(),
                preset.order,
                true,
                prompt_mode,
                uc_mode,
                before,
                after,
                replace,
                uc_before,
                uc_after,
                uc_replace,
                preset.quality_override.as_deref(),
                preset.uc_preset_override.as_deref(),
                preset.preview_thumb.as_ref().map(|value| value.id.as_str()),
                preset
                    .preview_thumb
                    .as_ref()
                    .and_then(|value| value.variant_id.as_ref())
                    .map(VariantId::as_str),
                i64::try_from(preset.created_at_ms)
                    .map_err(|error| PromptResourceError::repository(error.to_string()))?,
                i64::try_from(preset.updated_at_ms)
                    .map_err(|error| PromptResourceError::repository(error.to_string()))?,
            ],
        )
        .map(|_| ())
        .map_err(sql_error)
}

trait SqlExecutor {
    fn execute_sql<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize>;
}

impl SqlExecutor for rusqlite::Connection {
    fn execute_sql<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.execute(sql, params)
    }
}

impl SqlExecutor for rusqlite::Transaction<'_> {
    fn execute_sql<P: Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.execute(sql, params)
    }
}

fn prompt_chunk_select(where_clause: &str) -> String {
    format!(
        r"
        SELECT chunk_id, chunk_key, content, category, description,
               preview_resource_id, preview_variant_id, created_at_ms, updated_at_ms
        FROM prompt_chunks {where_clause}
        "
    )
}

fn prompt_chunk_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptChunk> {
    let preview_resource_id = row.get::<_, Option<String>>(5)?;
    let preview_variant_id = row.get::<_, Option<String>>(6)?;
    Ok(PromptChunk {
        id: PromptChunkId::new(row.get::<_, String>(0)?),
        key: PromptChunkKey::parse(&row.get::<_, String>(1)?).map_err(to_sql_error)?,
        content: row.get(2)?,
        category: row.get(3)?,
        description: row.get(4)?,
        preview_thumb: preview_resource_id.map(|id| {
            ResourceRef::new(ResourceId::new(id), preview_variant_id.map(VariantId::new))
        }),
        created_at_ms: i64_to_u64(row.get(7)?)?,
        updated_at_ms: i64_to_u64(row.get(8)?)?,
    })
}

fn prompt_preset_select(where_clause: &str) -> String {
    format!(
        r"
        SELECT preset_id, preset_kind, name, category, description, sort_order, enabled,
               prompt_mode, uc_mode, before_text, after_text, replace_text, uc_before_text,
               uc_after_text, uc_replace_text, quality_override, uc_preset_override,
               preview_resource_id, preview_variant_id, created_at_ms, updated_at_ms
        FROM prompt_presets {where_clause}
        "
    )
}

fn prompt_preset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptPreset> {
    let preview_resource_id = row.get::<_, Option<String>>(17)?;
    let preview_variant_id = row.get::<_, Option<String>>(18)?;
    let prompt_behavior = prompt_behavior_from_fields(
        &row.get::<_, String>(7)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
    )
    .map_err(to_sql_error)?;
    let uc_behavior = prompt_behavior_from_fields(
        &row.get::<_, String>(8)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
    )
    .map_err(to_sql_error)?;
    Ok(PromptPreset {
        id: PromptPresetId::new(row.get::<_, String>(0)?),
        kind: preset_kind_from_str(&row.get::<_, String>(1)?).map_err(to_sql_error)?,
        name: row.get(2)?,
        category: row.get(3)?,
        description: row.get(4)?,
        order: row.get(5)?,
        prompt_behavior,
        uc_behavior,
        quality_override: row.get(15)?,
        uc_preset_override: row.get(16)?,
        preview_thumb: preview_resource_id.map(|id| {
            ResourceRef::new(ResourceId::new(id), preview_variant_id.map(VariantId::new))
        }),
        created_at_ms: i64_to_u64(row.get(19)?)?,
        updated_at_ms: i64_to_u64(row.get(20)?)?,
    })
}

fn preset_behavior_fields(behavior: &PromptPresetBehavior) -> (&'static str, &str, &str, &str) {
    match behavior {
        PromptPresetBehavior::Surround { before, after } => ("surround", before, after, ""),
        PromptPresetBehavior::Replace { text } => ("replace", "", "", text),
    }
}

fn prompt_behavior_from_fields(
    mode: &str,
    before: String,
    after: String,
    replace: String,
) -> PromptResourceResult<PromptPresetBehavior> {
    match mode {
        "surround" => Ok(PromptPresetBehavior::Surround { before, after }),
        "replace" => Ok(PromptPresetBehavior::Replace { text: replace }),
        _ => Err(PromptResourceError::repository(format!(
            "unknown prompt preset behavior `{mode}`"
        ))),
    }
}

fn prompt_preset_text_fields(
    connection: &rusqlite::Transaction<'_>,
) -> PromptResourceResult<Vec<(String, &'static str, String)>> {
    let mut items = Vec::new();
    for column in [
        "before_text",
        "after_text",
        "replace_text",
        "uc_before_text",
        "uc_after_text",
        "uc_replace_text",
    ] {
        let sql = format!("SELECT preset_id, {column} FROM prompt_presets ORDER BY preset_id");
        let mut statement = connection.prepare(&sql).map_err(sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(sql_error)?;
        for row in rows {
            let (preset_id, content) = row.map_err(sql_error)?;
            items.push((preset_id, column, content));
        }
    }
    Ok(items)
}

const fn preset_kind_to_str(kind: PromptPresetKind) -> &'static str {
    match kind {
        PromptPresetKind::Main => "main",
        PromptPresetKind::Character => "character",
    }
}

fn preset_kind_from_str(value: &str) -> PromptResourceResult<PromptPresetKind> {
    match value {
        "main" => Ok(PromptPresetKind::Main),
        "character" => Ok(PromptPresetKind::Character),
        _ => Err(PromptResourceError::repository(format!(
            "unknown prompt preset kind `{value}`"
        ))),
    }
}

fn i64_to_u64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value)
        .map_err(|error| to_sql_error(PromptResourceError::repository(error.to_string())))
}

fn sql_error(error: rusqlite::Error) -> PromptResourceError {
    let message = error.to_string();
    drop(error);
    if message.contains("UNIQUE constraint failed") {
        PromptResourceError::conflict(message)
    } else {
        PromptResourceError::repository(message)
    }
}

fn prompt_error(error: DatabaseError) -> PromptResourceError {
    let message = error.to_string();
    drop(error);
    PromptResourceError::repository(message)
}

fn to_sql_error(error: PromptResourceError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
