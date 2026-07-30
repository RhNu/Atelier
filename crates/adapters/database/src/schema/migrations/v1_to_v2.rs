use rusqlite::{Connection, params};
use serde_json::{Value, json};

use crate::error::{DatabaseError, DatabaseResult};

const FROM_VERSION: i64 = 1;
const TO_VERSION: i64 = 2;

pub(super) fn migrate(connection: &mut Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        r"
        ALTER TABLE gallery_items
            ADD COLUMN safety_scan_state TEXT NOT NULL DEFAULT 'unscanned';
        CREATE INDEX idx_gallery_items_safety_scan_state
            ON gallery_items(safety_scan_state, indexed_at_ms, item_id);
        ",
    )?;

    rewrite_json_column(
        &transaction,
        "artifacts",
        "artifact_id",
        "record_json",
        |value| set_json_schema(value, TO_VERSION),
    )?;
    rewrite_json_column(
        &transaction,
        "vibe_documents",
        "vibe_id",
        "document_json",
        |value| set_json_schema(value, TO_VERSION),
    )?;
    rewrite_json_column(
        &transaction,
        "vibe_encodings",
        "cache_key",
        "record_json",
        |value| set_json_schema(value, TO_VERSION),
    )?;
    migrate_gallery_items(&transaction)?;
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = ?1 WHERE singleton = 1",
        params![TO_VERSION],
    )?;
    transaction.commit()?;
    Ok(TO_VERSION)
}

fn migrate_gallery_items(transaction: &rusqlite::Transaction<'_>) -> DatabaseResult<()> {
    let mut statement =
        transaction.prepare("SELECT item_id, item_json FROM gallery_items ORDER BY item_id")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for (item_id, text) in rows {
        let mut value: Value = serde_json::from_str(&text).map_err(json_error)?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| migration_error("gallery item JSON is not an object"))?;
        let legacy_assessment = object.remove("safety_assessment").unwrap_or(Value::Null);
        let (state, safety) = if legacy_assessment.is_null() {
            ("unscanned", json!({ "state": "unscanned" }))
        } else {
            (
                "unavailable",
                json!({
                    "state": "unavailable",
                    "message": "legacy safety assessment requires rescan"
                }),
            )
        };
        object.insert("schema_version".to_owned(), Value::from(TO_VERSION));
        object.insert("safety".to_owned(), safety);
        let migrated = serde_json::to_string(&value).map_err(json_error)?;
        transaction.execute(
            r"
            UPDATE gallery_items
            SET item_json = ?2,
                safety_scan_state = ?3,
                effective_safety_label = manual_safety_override
            WHERE item_id = ?1
            ",
            params![item_id, migrated, state],
        )?;
    }
    Ok(())
}

fn rewrite_json_column(
    transaction: &rusqlite::Transaction<'_>,
    table: &str,
    key_column: &str,
    json_column: &str,
    migrate: impl Fn(Value) -> DatabaseResult<Value>,
) -> DatabaseResult<()> {
    let select = format!("SELECT {key_column}, {json_column} FROM {table} ORDER BY {key_column}");
    let mut statement = transaction.prepare(&select)?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    let update = format!("UPDATE {table} SET {json_column} = ?2 WHERE {key_column} = ?1");
    for (key, text) in rows {
        let value = serde_json::from_str(&text).map_err(json_error)?;
        let migrated = serde_json::to_string(&migrate(value)?).map_err(json_error)?;
        transaction.execute(&update, params![key, migrated])?;
    }
    Ok(())
}

fn set_json_schema(mut value: Value, version: i64) -> DatabaseResult<Value> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| migration_error("stored JSON is not an object"))?;
    object.insert("schema_version".to_owned(), Value::from(version));
    Ok(value)
}

fn json_error(error: impl std::fmt::Display) -> DatabaseError {
    migration_error(error.to_string())
}

fn migration_error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(format!(
        "database migration {FROM_VERSION}->{TO_VERSION} failed: {}",
        message.into()
    ))
}
