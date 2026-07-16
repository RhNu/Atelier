use super::run_history::upsert_run_history_row;
use super::{
    DatabaseConnection, JobQueueRepository, JobQueueSnapshot, JobQueueSnapshotDto, JobResult,
    OptionalExtension, RunHistoryRecord, async_trait, decode_json, encode_json, job_store_error,
    now_ms, params,
};

#[derive(Clone, Debug)]
pub struct DatabaseJobQueueRepository {
    connection: DatabaseConnection,
}

impl DatabaseJobQueueRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Atomically commits the active queue snapshot and its derived run-history
    /// records in the same `SQLite` transaction.
    ///
    /// # Errors
    /// Returns an error when encoding or committing either state fails.
    pub fn commit_queue_and_history(
        &self,
        snapshot: Option<&JobQueueSnapshot>,
        history: Vec<RunHistoryRecord>,
    ) -> JobResult<()> {
        let encoded = snapshot
            .map(|snapshot| encode_json(&JobQueueSnapshotDto::from_domain(snapshot)))
            .transpose()
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

#[async_trait]
impl JobQueueRepository for DatabaseJobQueueRepository {
    async fn load_queue_snapshot(&self) -> JobResult<Option<JobQueueSnapshot>> {
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

    async fn save_queue_snapshot(&self, snapshot: &JobQueueSnapshot) -> JobResult<()> {
        let json =
            encode_json(&JobQueueSnapshotDto::from_domain(snapshot)).map_err(job_store_error)?;
        {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
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
        }
        Ok(())
    }

    async fn clear_queue_snapshot(&self) -> JobResult<()> {
        {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
                .execute(
                    "DELETE FROM generation_queue_state WHERE state_key = 'active'",
                    [],
                )
                .map_err(job_store_error)?;
        }
        Ok(())
    }
}
