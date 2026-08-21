use rusqlite::{Connection, params};
use serde_json::{Map, Value};

use crate::error::{DatabaseError, DatabaseResult};

const V45_FULL: &str = "nai-diffusion-4-5-full";

pub(super) fn migrate(connection: &mut Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        r"
        CREATE TABLE prompt_chunk_models (
            chunk_id TEXT NOT NULL,
            model TEXT NOT NULL,
            PRIMARY KEY (chunk_id, model),
            FOREIGN KEY (chunk_id) REFERENCES prompt_chunks(chunk_id) ON DELETE CASCADE
        );
        CREATE INDEX idx_prompt_chunk_models_model
            ON prompt_chunk_models(model, chunk_id);
        CREATE TABLE prompt_preset_models (
            preset_id TEXT NOT NULL,
            model TEXT NOT NULL,
            PRIMARY KEY (preset_id, model),
            FOREIGN KEY (preset_id) REFERENCES prompt_presets(preset_id) ON DELETE CASCADE
        );
        CREATE INDEX idx_prompt_preset_models_model
            ON prompt_preset_models(model, preset_id);
        INSERT INTO prompt_chunk_models(chunk_id, model)
            SELECT chunk_id, 'nai-diffusion-4-5-full' FROM prompt_chunks;
        INSERT INTO prompt_preset_models(preset_id, model)
            SELECT preset_id, 'nai-diffusion-4-5-full' FROM prompt_presets;
        ",
    )?;

    migrate_workspace_settings(&transaction)?;
    migrate_generation_payloads(&transaction)?;
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = 4 WHERE singleton = 1",
        [],
    )?;
    transaction.commit()?;
    Ok(4)
}

fn migrate_workspace_settings(transaction: &rusqlite::Transaction<'_>) -> DatabaseResult<()> {
    let mut statement = transaction
        .prepare("SELECT setting_key, value_json FROM workspace_settings ORDER BY setting_key")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for (key, json) in rows {
        if key != "workspace" && key != "generation.draft" {
            continue;
        }
        let mut value = parse_json(&json, &format!("workspace setting `{key}`"))?;
        if key == "generation.draft" {
            migrate_draft(&mut value)?;
        } else {
            migrate_value(&mut value, false);
        }
        transaction.execute(
            "UPDATE workspace_settings SET value_json = ?2 WHERE setting_key = ?1",
            params![key, encode_json(&value)?],
        )?;
    }
    Ok(())
}

fn migrate_generation_payloads(transaction: &rusqlite::Transaction<'_>) -> DatabaseResult<()> {
    let mut statement = transaction.prepare(
        "SELECT payload_kind, payload_ref, payload_json FROM generation_payloads ORDER BY payload_kind, payload_ref",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for (kind, reference, json) in rows {
        let mut value = parse_json(&json, &format!("generation payload `{kind}:{reference}`"))?;
        migrate_value(&mut value, true);
        transaction.execute(
            "UPDATE generation_payloads SET payload_json = ?3 WHERE payload_kind = ?1 AND payload_ref = ?2",
            params![kind, reference, encode_json(&value)?],
        )?;
    }
    Ok(())
}

fn migrate_draft(value: &mut Value) -> DatabaseResult<()> {
    let object = value.as_object_mut().ok_or_else(|| {
        DatabaseError::new("generation draft JSON must be an object during schema v4 migration")
    })?;
    let model = fix_model_string(
        object
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(V45_FULL),
    );
    let mut state = Map::new();
    state.insert("model".to_owned(), Value::String(model.clone()));
    for field in [
        "main_preset_id",
        "prompt",
        "negative_prompt",
        "characters",
        "character_position_mode",
    ] {
        if let Some(field_value) = object.remove(field) {
            state.insert(field.to_owned(), field_value);
        }
    }
    state
        .entry("main_preset_id".to_owned())
        .or_insert(Value::Null);
    state
        .entry("prompt".to_owned())
        .or_insert_with(|| Value::String(String::new()));
    state
        .entry("negative_prompt".to_owned())
        .or_insert_with(|| Value::String(String::new()));
    state
        .entry("characters".to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    state
        .entry("character_position_mode".to_owned())
        .or_insert_with(|| Value::String("global".to_owned()));
    object.insert(
        "prompt_states".to_owned(),
        Value::Array(vec![Value::Object(state)]),
    );
    if let Some(slots) = object
        .get_mut("vibe")
        .and_then(Value::as_object_mut)
        .and_then(|vibe| vibe.get_mut("slots"))
        .and_then(Value::as_array_mut)
    {
        for slot in slots {
            if let Some(slot) = slot.as_object_mut() {
                slot.insert("model".to_owned(), Value::String(model.clone()));
            }
        }
    }
    migrate_value(value, false);
    Ok(())
}

fn migrate_value(value: &mut Value, protocol_fields: bool) {
    match value {
        Value::Array(items) => items
            .iter_mut()
            .for_each(|item| migrate_value(item, protocol_fields)),
        Value::Object(object) => {
            if object.get("schema_version").and_then(Value::as_u64) == Some(1) {
                object.insert("schema_version".to_owned(), Value::from(2));
            }
            if let Some(Value::String(model)) = object.get_mut("model") {
                *model = fix_model_string(model);
            }
            if let Some(quality) = object.get_mut("quality")
                && let Some(enabled) = quality.as_bool()
            {
                *quality = Value::String(if enabled { "standard" } else { "none" }.to_owned());
            }
            if object.contains_key("quality") {
                object
                    .entry("transparent_background".to_owned())
                    .or_insert(Value::Bool(false));
            }
            if protocol_fields {
                if let Some(img2img) = object.remove("i2i") {
                    object.insert("img2img".to_owned(), img2img);
                }
                if let Some(mut controlnet) = object.remove("controlnet") {
                    if let Some(config) = controlnet.as_object_mut()
                        && let Some(mut images) = config.remove("images")
                    {
                        if let Some(items) = images.as_array_mut() {
                            for item in items {
                                if let Some(item) = item.as_object_mut() {
                                    item.remove("info_extracted");
                                }
                            }
                        }
                        config.insert("references".to_owned(), images);
                    }
                    object.insert("vibe_transfer".to_owned(), controlnet);
                }
            }
            for child in object.values_mut() {
                migrate_value(child, protocol_fields);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn fix_model_string(value: &str) -> String {
    if value == "nai-diffusion-3-furry" {
        "nai-diffusion-furry-3".to_owned()
    } else {
        value.to_owned()
    }
}

fn parse_json(value: &str, label: &str) -> DatabaseResult<Value> {
    serde_json::from_str(value)
        .map_err(|error| DatabaseError::new(format!("invalid {label} JSON: {error}")))
}

fn encode_json(value: &Value) -> DatabaseResult<String> {
    serde_json::to_string(value).map_err(|error| DatabaseError::new(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v3_connection() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                r"
                PRAGMA foreign_keys = ON;
                CREATE TABLE atelier_schema (singleton INTEGER PRIMARY KEY, schema_version INTEGER NOT NULL);
                INSERT INTO atelier_schema VALUES (1, 3);
                CREATE TABLE prompt_chunks (chunk_id TEXT PRIMARY KEY);
                CREATE TABLE prompt_presets (preset_id TEXT PRIMARY KEY);
                CREATE TABLE workspace_settings (setting_key TEXT PRIMARY KEY, value_json TEXT NOT NULL);
                CREATE TABLE generation_payloads (
                    payload_kind TEXT NOT NULL,
                    payload_ref TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    PRIMARY KEY (payload_kind, payload_ref)
                );
                INSERT INTO prompt_chunks VALUES ('chunk-1');
                INSERT INTO prompt_presets VALUES ('preset-1');
                ",
            )
            .unwrap();
        connection
    }

    #[test]
    fn backfills_model_bindings_and_rewrites_json() {
        let mut connection = v3_connection();
        connection.execute(
            "INSERT INTO workspace_settings VALUES ('generation.draft', ?1)",
            [r#"{"schema_version":1,"model":"nai-diffusion-3-furry","main_preset_id":null,"prompt":"中文 😀","negative_prompt":"","characters":[],"character_position_mode":"global","quality":true,"i2i":null,"vibe":{"slots":[{"id":"slot"}]}}"#],
        ).unwrap();
        connection.execute(
            "INSERT INTO generation_payloads VALUES ('submitted', 'job-1', ?1)",
            [r#"{"schema_version":1,"request":{"quality":false,"i2i":{"strength":0.5},"controlnet":{"images":[{"encoding":"abc","info_extracted":0.7,"strength":1.0}]}}}"#],
        ).unwrap();

        assert_eq!(migrate(&mut connection).unwrap(), 4);
        assert_eq!(
            connection
                .query_row("SELECT model FROM prompt_chunk_models", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            V45_FULL
        );
        assert_eq!(
            connection
                .query_row("SELECT model FROM prompt_preset_models", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            V45_FULL
        );
        let draft: Value = serde_json::from_str(&connection.query_row(
            "SELECT value_json FROM workspace_settings WHERE setting_key = 'generation.draft'",
            [], |row| row.get::<_, String>(0),
        ).unwrap()).unwrap();
        assert_eq!(draft["model"], "nai-diffusion-furry-3");
        assert_eq!(draft["quality"], "standard");
        assert_eq!(draft["prompt_states"][0]["prompt"], "中文 😀");
        assert_eq!(draft["vibe"]["slots"][0]["model"], "nai-diffusion-furry-3");
        assert!(draft.get("i2i").is_some());

        let payload: Value = serde_json::from_str(
            &connection
                .query_row("SELECT payload_json FROM generation_payloads", [], |row| {
                    row.get::<_, String>(0)
                })
                .unwrap(),
        )
        .unwrap();
        assert_eq!(payload["request"]["quality"], "none");
        assert!(payload["request"].get("img2img").is_some());
        assert!(payload["request"].get("i2i").is_none());
        assert!(
            payload["request"]["vibe_transfer"]["references"][0]
                .get("info_extracted")
                .is_none()
        );
    }

    #[test]
    fn malformed_json_rolls_back_the_whole_migration() {
        let mut connection = v3_connection();
        connection
            .execute(
                "INSERT INTO workspace_settings VALUES ('workspace', '{broken')",
                [],
            )
            .unwrap();

        assert!(migrate(&mut connection).is_err());
        assert_eq!(
            connection
                .query_row("SELECT schema_version FROM atelier_schema", [], |row| row
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            3
        );
        let table_count = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'prompt_chunk_models'",
            [], |row| row.get::<_, i64>(0),
        ).unwrap();
        assert_eq!(table_count, 0);
    }
}
