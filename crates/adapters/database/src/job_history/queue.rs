use super::run_history::upsert_run_history_row;
use super::{
    DatabaseConnection, GenerationStore, JobQueueSnapshot, JobQueueSnapshotDto, JobResult,
    OptionalExtension, RunHistoryRecord, async_trait, decode_json, encode_json, job_store_error,
    now_ms, params,
};

#[derive(Clone, Debug)]
pub struct DatabaseGenerationStore {
    connection: DatabaseConnection,
}

impl DatabaseGenerationStore {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl GenerationStore for DatabaseGenerationStore {
    async fn load(&self) -> JobResult<Option<JobQueueSnapshot>> {
        let json = {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
                .query_row(
                    "SELECT snapshot_json FROM generation_queue_state WHERE state_key = 'active'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(job_store_error)?
        };
        json.map(|json| {
            decode_json::<JobQueueSnapshotDto>(&json)
                .map_err(job_store_error)?
                .into_domain()
        })
        .transpose()
    }

    /// Atomically commits the active queue snapshot and its derived run-history
    /// records in the same `SQLite` transaction.
    ///
    /// # Errors
    /// Returns an error when encoding or committing either state fails.
    async fn commit(
        &self,
        snapshot: Option<&JobQueueSnapshot>,
        history: Vec<RunHistoryRecord>,
    ) -> JobResult<()> {
        let encoded = snapshot
            .map(|snapshot| encode_json(&JobQueueSnapshotDto::from_domain(snapshot)))
            .transpose()
            .map_err(job_store_error)?;
        let _gate = self
            .connection
            .acquire_transaction_gate()
            .await
            .map_err(job_store_error)?;
        let mut connection = self.connection.lock().map_err(job_store_error)?;
        let transaction = connection.transaction().map_err(job_store_error)?;
        if let Some(json) = encoded {
            transaction
                .execute(
                    r"
                    INSERT INTO generation_queue_state(state_key, snapshot_json, updated_at_ms)
                    VALUES ('active', ?1, ?2)
                    ON CONFLICT(state_key) DO UPDATE SET
                        snapshot_json = excluded.snapshot_json,
                        updated_at_ms = excluded.updated_at_ms
                    ",
                    params![json, i64::try_from(now_ms()).unwrap_or(i64::MAX)],
                )
                .map_err(job_store_error)?;
        } else {
            transaction
                .execute(
                    "DELETE FROM generation_queue_state WHERE state_key = 'active'",
                    [],
                )
                .map_err(job_store_error)?;
        }
        for record in history {
            upsert_run_history_row(&transaction, &record).map_err(job_store_error)?;
        }
        transaction.commit().map_err(job_store_error)
    }
}
