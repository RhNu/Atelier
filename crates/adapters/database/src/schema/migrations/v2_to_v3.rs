use rusqlite::{Connection, params};

use crate::error::DatabaseResult;

const TO_VERSION: i64 = 3;

pub(super) fn migrate(connection: &mut Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(
        r"
        DROP INDEX IF EXISTS idx_api_key_records_active;
        DROP TABLE IF EXISTS api_key_records;
        ",
    )?;
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = ?1 WHERE singleton = 1",
        params![TO_VERSION],
    )?;
    transaction.commit()?;
    Ok(TO_VERSION)
}
