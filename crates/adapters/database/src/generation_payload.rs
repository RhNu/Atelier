#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use nai_atelier_jobs::JobPayloadRef;
use nai_atelier_kernel::{
    GenerationPayloadStore, KernelError, KernelResult, PreparedGenerationPayload,
    SubmittedGenerationPayload,
};
use rusqlite::{OptionalExtension, params};

use crate::codec::JsonCodec;
use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;
use crate::generation_codec::{PreparedGenerationPayloadDto, SubmittedGenerationPayloadDto};

#[derive(Clone, Debug)]
pub struct DatabaseGenerationPayloadStore {
    connection: DatabaseConnection,
}

impl DatabaseGenerationPayloadStore {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl GenerationPayloadStore for DatabaseGenerationPayloadStore {
    async fn save_submitted_payload(
        &self,
        payload: SubmittedGenerationPayload,
    ) -> KernelResult<()> {
        let json = SubmittedGenerationPayloadDto::encode_domain(&payload).map_err(kernel_error)?;
        let connection = self.connection.lock().map_err(kernel_error)?;
        connection
            .execute(
                r"
                INSERT INTO generation_payloads(payload_ref, payload_kind, payload_json)
                VALUES (?1, 'submitted', ?2)
                ON CONFLICT(payload_kind, payload_ref) DO UPDATE
                SET payload_json = excluded.payload_json
                ",
                params![payload.payload_ref.as_str(), json],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn get_submitted_payload(
        &self,
        payload_ref: &JobPayloadRef,
    ) -> KernelResult<Option<SubmittedGenerationPayload>> {
        let connection = self.connection.lock().map_err(kernel_error)?;
        let json = connection
            .query_row(
                r"
                SELECT payload_json
                FROM generation_payloads
                WHERE payload_ref = ?1 AND payload_kind = 'submitted'
                ",
                params![payload_ref.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?;
        json.map(|text| SubmittedGenerationPayloadDto::decode_domain(&text).map_err(kernel_error))
            .transpose()
    }

    async fn save_prepared_payload(&self, payload: PreparedGenerationPayload) -> KernelResult<()> {
        let json = PreparedGenerationPayloadDto::encode_domain(&payload).map_err(kernel_error)?;
        let connection = self.connection.lock().map_err(kernel_error)?;
        connection
            .execute(
                r"
                INSERT INTO generation_payloads(payload_ref, payload_kind, payload_json)
                VALUES (?1, 'prepared', ?2)
                ON CONFLICT(payload_kind, payload_ref) DO UPDATE
                SET payload_json = excluded.payload_json
                ",
                params![payload.payload_ref.as_str(), json],
            )
            .map(|_| ())
            .map_err(sql_error)
    }
}

fn sql_error(error: rusqlite::Error) -> KernelError {
    let message = error.to_string();
    drop(error);
    KernelError::PayloadStore(message)
}

fn kernel_error(error: DatabaseError) -> KernelError {
    let message = error.to_string();
    drop(error);
    KernelError::PayloadStore(message)
}
