use rusqlite::params;
use serde_json::{Value, json};

use crate::error::{DatabaseError, DatabaseResult};

const TO_VERSION: i64 = 8;

pub(super) fn migrate(connection: &mut rusqlite::Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    let mut statement = transaction
        .prepare("SELECT session_id, model_json FROM agent_sessions ORDER BY session_id ASC")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    for (session_id, text) in rows {
        let mut value: Value = serde_json::from_str(&text).map_err(|error| {
            DatabaseError::new(format!(
                "invalid Agent model snapshot for session `{session_id}`: {error}"
            ))
        })?;
        let model = value.as_object_mut().ok_or_else(|| {
            DatabaseError::new(format!(
                "Agent model snapshot for session `{session_id}` must be an object"
            ))
        })?;
        let image_input = if model
            .remove("supports_vision")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            "tool_result"
        } else {
            "none"
        };
        model.insert(
            "capabilities".to_owned(),
            json!({ "image_input": image_input }),
        );
        transaction.execute(
            "UPDATE agent_sessions SET model_json = ?2 WHERE session_id = ?1",
            params![session_id, serde_json::to_string(&value)?],
        )?;
    }
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = ?1 WHERE singleton = 1",
        [TO_VERSION],
    )?;
    transaction.commit()?;
    Ok(TO_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_agent_model_capabilities() {
        let mut connection = rusqlite::Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE atelier_schema (
                    singleton INTEGER PRIMARY KEY,
                    schema_version INTEGER NOT NULL
                );
                INSERT INTO atelier_schema(singleton, schema_version) VALUES (1, 7);
                CREATE TABLE agent_sessions (
                    session_id TEXT PRIMARY KEY,
                    model_json TEXT NOT NULL
                );
                INSERT INTO agent_sessions(session_id, model_json) VALUES (
                    'session',
                    '{"model_id":"m","supports_vision":true}'
                );
                "#,
            )
            .expect("schema");

        assert_eq!(migrate(&mut connection).expect("migration"), 8);
        let model_json: String = connection
            .query_row(
                "SELECT model_json FROM agent_sessions WHERE session_id = 'session'",
                [],
                |row| row.get(0),
            )
            .expect("model");
        let model: Value = serde_json::from_str(&model_json).expect("json");
        assert_eq!(model["capabilities"]["image_input"], "tool_result");
        assert!(model.get("supports_vision").is_none());
        let version: i64 = connection
            .query_row(
                "SELECT schema_version FROM atelier_schema WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("version");
        assert_eq!(version, 8);
    }
}
