use super::{
    DatabaseConnection, JobResult, OptionalExtension, RunHistoryQuery, RunHistoryRecord,
    RunHistoryRepository, RunOutputRecord, async_trait, job_store_error, params,
    run_history_from_row, run_history_kind_as_str, run_history_status_as_str,
};

#[derive(Clone, Debug)]
pub struct DatabaseRunHistoryRepository {
    connection: DatabaseConnection,
}

impl DatabaseRunHistoryRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl RunHistoryRepository for DatabaseRunHistoryRepository {
    async fn upsert_run_history(&self, record: RunHistoryRecord) -> JobResult<()> {
        let created_at_ms = i64::try_from(record.created_at_ms).unwrap_or(i64::MAX);
        let updated_at_ms = i64::try_from(record.updated_at_ms).unwrap_or(i64::MAX);
        let completed_at_ms = record
            .completed_at_ms
            .map(|value| i64::try_from(value).unwrap_or(i64::MAX));
        let recoverable = i64::from(record.recoverable);
        {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
                .execute(
                    r"
                    INSERT INTO run_history(
                        run_id,
                        run_kind,
                        run_status,
                        batch_id,
                        job_id,
                        origin_run_id,
                        submitted_payload_ref,
                        prepared_payload_ref,
                        title,
                        last_error,
                        created_at_ms,
                        updated_at_ms,
                        completed_at_ms,
                        recoverable
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                    ON CONFLICT(run_id) DO UPDATE SET
                        run_kind = excluded.run_kind,
                        run_status = excluded.run_status,
                        batch_id = excluded.batch_id,
                        job_id = excluded.job_id,
                        origin_run_id = excluded.origin_run_id,
                        submitted_payload_ref = excluded.submitted_payload_ref,
                        prepared_payload_ref = excluded.prepared_payload_ref,
                        title = excluded.title,
                        last_error = excluded.last_error,
                        created_at_ms = excluded.created_at_ms,
                        updated_at_ms = excluded.updated_at_ms,
                        completed_at_ms = excluded.completed_at_ms,
                        recoverable = excluded.recoverable
                    ",
                    params![
                        record.run_id,
                        run_history_kind_as_str(record.kind),
                        run_history_status_as_str(record.status),
                        record.batch_id,
                        record.job_id,
                        record.origin_run_id,
                        record.submitted_payload_ref,
                        record.prepared_payload_ref,
                        record.title,
                        record.last_error,
                        created_at_ms,
                        updated_at_ms,
                        completed_at_ms,
                        recoverable,
                    ],
                )
                .map_err(job_store_error)?;
        }
        Ok(())
    }

    async fn get_run_history(&self, run_id: &str) -> JobResult<Option<RunHistoryRecord>> {
        let connection = self.connection.lock().map_err(job_store_error)?;
        connection
            .query_row(
                r"
                SELECT
                    run_id,
                    run_kind,
                    run_status,
                    batch_id,
                    job_id,
                    origin_run_id,
                    submitted_payload_ref,
                    prepared_payload_ref,
                    title,
                    last_error,
                    created_at_ms,
                    updated_at_ms,
                    completed_at_ms,
                    recoverable
                FROM run_history
                WHERE run_id = ?1
                ",
                params![run_id],
                run_history_from_row,
            )
            .optional()
            .map_err(job_store_error)
    }

    async fn query_run_history(&self, query: RunHistoryQuery) -> JobResult<Vec<RunHistoryRecord>> {
        let records = {
            let connection = self.connection.lock().map_err(job_store_error)?;
            let mut statement = connection
                .prepare(
                    r"
                    SELECT
                        run_id,
                        run_kind,
                        run_status,
                        batch_id,
                        job_id,
                        origin_run_id,
                        submitted_payload_ref,
                        prepared_payload_ref,
                        title,
                        last_error,
                        created_at_ms,
                        updated_at_ms,
                        completed_at_ms,
                        recoverable
                    FROM run_history
                    WHERE (?1 IS NULL OR run_kind = ?1)
                        AND (?2 IS NULL OR run_status = ?2)
                    ORDER BY updated_at_ms DESC, run_id ASC
                    LIMIT ?3 OFFSET ?4
                    ",
                )
                .map_err(job_store_error)?;
            let rows = statement
                .query_map(
                    params![
                        query.kind.map(run_history_kind_as_str),
                        query.status.map(run_history_status_as_str),
                        i64::try_from(query.limit).unwrap_or(i64::MAX),
                        i64::try_from(query.offset).unwrap_or(i64::MAX),
                    ],
                    run_history_from_row,
                )
                .map_err(job_store_error)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(job_store_error)?
        };
        Ok(records)
    }

    async fn count_run_history(&self, query: RunHistoryQuery) -> JobResult<usize> {
        let count = {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
                .query_row(
                    r"
                    SELECT COUNT(*)
                    FROM run_history
                    WHERE (?1 IS NULL OR run_kind = ?1)
                        AND (?2 IS NULL OR run_status = ?2)
                    ",
                    params![
                        query.kind.map(run_history_kind_as_str),
                        query.status.map(run_history_status_as_str),
                    ],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(job_store_error)?
        };
        usize::try_from(count).map_err(job_store_error)
    }

    async fn run_history_batch_exists(&self, batch_id: &str) -> JobResult<bool> {
        let exists = {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM run_history WHERE batch_id = ?1)",
                    params![batch_id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(job_store_error)?
        };
        Ok(exists)
    }

    async fn delete_run_history_items(&self, run_ids: &[String]) -> JobResult<usize> {
        if run_ids.is_empty() {
            return Ok(0);
        }

        let mut deleted = 0;
        {
            let mut connection = self.connection.lock().map_err(job_store_error)?;
            let transaction = connection.transaction().map_err(job_store_error)?;
            for run_id in run_ids {
                deleted += transaction
                    .execute("DELETE FROM run_history WHERE run_id = ?1", params![run_id])
                    .map_err(job_store_error)?;
            }
            transaction.commit().map_err(job_store_error)?;
        }
        Ok(deleted)
    }

    async fn upsert_run_output(&self, output: RunOutputRecord) -> JobResult<()> {
        {
            let connection = self.connection.lock().map_err(job_store_error)?;
            connection
                .execute(
                    r"
                    INSERT OR REPLACE INTO run_outputs(
                        run_id,
                        artifact_id,
                        item_id,
                        resource_id,
                        variant_id,
                        asset_role,
                        variant_kind
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ",
                    params![
                        output.run_id,
                        output.artifact_id,
                        output.item_id,
                        output.resource_id,
                        output.variant_id,
                        output.asset_role,
                        output.variant_kind,
                    ],
                )
                .map_err(job_store_error)?;
        }
        Ok(())
    }

    async fn list_run_outputs(&self, run_id: &str) -> JobResult<Vec<RunOutputRecord>> {
        let outputs = {
            let connection = self.connection.lock().map_err(job_store_error)?;
            let mut statement = connection
                .prepare(
                    r"
                    SELECT
                        run_id,
                        artifact_id,
                        item_id,
                        resource_id,
                        variant_id,
                        asset_role,
                        variant_kind
                    FROM run_outputs
                    WHERE run_id = ?1
                    ORDER BY
                        artifact_id ASC,
                        CASE asset_role
                            WHEN 'original' THEN 0
                            WHEN 'thumbnail' THEN 1
                            WHEN 'preview' THEN 2
                            WHEN 'sanitized' THEN 3
                            WHEN 'export' THEN 4
                            ELSE 5
                        END ASC,
                        resource_id ASC,
                        variant_id ASC
                    ",
                )
                .map_err(job_store_error)?;
            let rows = statement
                .query_map(params![run_id], |row| {
                    Ok(RunOutputRecord {
                        run_id: row.get(0)?,
                        artifact_id: row.get(1)?,
                        item_id: row.get(2)?,
                        resource_id: row.get(3)?,
                        variant_id: row.get(4)?,
                        asset_role: row.get(5)?,
                        variant_kind: row.get(6)?,
                    })
                })
                .map_err(job_store_error)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(job_store_error)?
        };
        Ok(outputs)
    }
}
