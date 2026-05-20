use async_trait::async_trait;
use nai_atelier_secrets::{
    ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, SecretRecordId, SecretsError, SecretsResult,
};
use rusqlite::{ErrorCode, OptionalExtension, params};

use crate::DatabaseConnection;

#[derive(Clone, Debug)]
pub struct DatabaseApiKeyRegistryStore {
    connection: DatabaseConnection,
}

impl DatabaseApiKeyRegistryStore {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl ApiKeyRegistryStore for DatabaseApiKeyRegistryStore {
    async fn insert_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        let mut connection = self.connection.lock().map_err(metadata_error)?;
        let tx = connection.transaction().map_err(metadata_error)?;
        if record.is_active {
            tx.execute("UPDATE api_key_records SET is_active = 0", [])
                .map_err(metadata_error)?;
        }
        if let Err(error) = tx.execute(
            r"
            INSERT INTO api_key_records(id, display_name, secret_record_id, is_active)
            VALUES (?1, ?2, ?3, ?4)
            ",
            params![
                record.id.as_str(),
                record.display_name,
                record.secret_record_id.as_str(),
                i64::from(record.is_active),
            ],
        ) {
            return Err(insert_error(error));
        }
        tx.commit().map_err(metadata_error)?;
        drop(connection);
        Ok(())
    }

    async fn save_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        let mut connection = self.connection.lock().map_err(metadata_error)?;
        let tx = connection.transaction().map_err(metadata_error)?;
        if record.is_active {
            tx.execute("UPDATE api_key_records SET is_active = 0", [])
                .map_err(metadata_error)?;
        }
        tx.execute(
            r"
            INSERT INTO api_key_records(id, display_name, secret_record_id, is_active)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                display_name = excluded.display_name,
                secret_record_id = excluded.secret_record_id,
                is_active = excluded.is_active
            ",
            params![
                record.id.as_str(),
                record.display_name,
                record.secret_record_id.as_str(),
                i64::from(record.is_active),
            ],
        )
        .map_err(metadata_error)?;
        tx.commit().map_err(metadata_error)?;
        drop(connection);
        Ok(())
    }

    async fn get_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<Option<ApiKeyRecord>> {
        let connection = self.connection.lock().map_err(metadata_error)?;
        connection
            .query_row(
                r"
                SELECT id, display_name, secret_record_id, is_active
                FROM api_key_records
                WHERE id = ?1
                ",
                params![id.as_str()],
                record_from_row,
            )
            .optional()
            .map_err(metadata_error)
    }

    async fn list_api_key_records(&self) -> SecretsResult<Vec<ApiKeyRecord>> {
        let connection = self.connection.lock().map_err(metadata_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT id, display_name, secret_record_id, is_active
                FROM api_key_records
                ORDER BY id ASC
                ",
            )
            .map_err(metadata_error)?;
        let rows = statement
            .query_map([], record_from_row)
            .map_err(metadata_error)?;
        let records = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(metadata_error)?;
        drop(statement);
        drop(connection);
        Ok(records)
    }

    async fn delete_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<bool> {
        let connection = self.connection.lock().map_err(metadata_error)?;
        let changed = connection
            .execute(
                "DELETE FROM api_key_records WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(metadata_error)?;
        drop(connection);
        Ok(changed > 0)
    }

    async fn set_active_api_key(&self, id: &ApiKeyId) -> SecretsResult<()> {
        let mut connection = self.connection.lock().map_err(metadata_error)?;
        let tx = connection.transaction().map_err(metadata_error)?;
        tx.execute("UPDATE api_key_records SET is_active = 0", [])
            .map_err(metadata_error)?;
        let changed = tx
            .execute(
                "UPDATE api_key_records SET is_active = 1 WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(metadata_error)?;
        if changed == 0 {
            drop(tx);
            drop(connection);
            return Err(SecretsError::metadata_store("api key does not exist"));
        }
        tx.commit().map_err(metadata_error)?;
        drop(connection);
        Ok(())
    }

    async fn get_active_api_key_record(&self) -> SecretsResult<Option<ApiKeyRecord>> {
        let connection = self.connection.lock().map_err(metadata_error)?;
        connection
            .query_row(
                r"
                SELECT id, display_name, secret_record_id, is_active
                FROM api_key_records
                WHERE is_active = 1
                ",
                [],
                record_from_row,
            )
            .optional()
            .map_err(metadata_error)
    }
}

fn record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiKeyRecord> {
    Ok(ApiKeyRecord {
        id: ApiKeyId::new(row.get::<_, String>(0)?),
        display_name: row.get(1)?,
        secret_record_id: SecretRecordId::new(row.get::<_, String>(2)?),
        is_active: row.get::<_, i64>(3)? != 0,
    })
}

fn metadata_error(error: impl std::fmt::Display) -> SecretsError {
    SecretsError::metadata_store(error.to_string())
}

fn insert_error(error: rusqlite::Error) -> SecretsError {
    if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) {
        SecretsError::validation("api key id already exists")
    } else {
        metadata_error(error)
    }
}
