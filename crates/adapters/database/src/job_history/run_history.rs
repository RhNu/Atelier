use super::{
    DatabaseConnection, GenerationBatchHistoryQuery, GenerationBatchHistoryRecord, JobResult,
    OptionalExtension, RunHistoryQuery, RunHistoryRecord, RunHistoryRepository, RunOutputRecord,
    async_trait, generation_batch_history_status_as_str, generation_batch_history_status_from_str,
    job_store_error, params, run_history_from_row, run_history_kind_as_str,
    run_history_status_as_str,
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
        self.upsert_run_history_batch(vec![record]).await
    }

    async fn upsert_run_history_batch(&self, records: Vec<RunHistoryRecord>) -> JobResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut connection = self.connection.lock().map_err(job_store_error)?;
        let transaction = connection.transaction().map_err(job_store_error)?;
        for record in records {
            upsert_run_history_row(&transaction, &record).map_err(job_store_error)?;
        }
        transaction.commit().map_err(job_store_error)
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
                    request_index,
                    expected_samples,
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
                        request_index,
                        expected_samples,
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

    async fn query_generation_batches(
        &self,
        query: GenerationBatchHistoryQuery,
    ) -> JobResult<Vec<GenerationBatchHistoryRecord>> {
        let connection = self.connection.lock().map_err(job_store_error)?;
        let sql = format!(
            "{GENERATION_BATCHES_CTE}\n\
             SELECT batch_id, batch_status, title, last_error, created_at_ms, updated_at_ms, \
                    completed_at_ms, request_count, completed_request_count, expected_sample_count \
             FROM generation_batches \
             WHERE (?1 IS NULL OR batch_status = ?1) \
             ORDER BY updated_at_ms DESC, batch_id ASC \
             LIMIT ?2 OFFSET ?3"
        );
        let mut statement = connection.prepare(&sql).map_err(job_store_error)?;
        let rows = statement
            .query_map(
                params![
                    query.status.map(generation_batch_history_status_as_str),
                    i64::try_from(query.limit).unwrap_or(i64::MAX),
                    i64::try_from(query.offset).unwrap_or(i64::MAX),
                ],
                generation_batch_history_from_row,
            )
            .map_err(job_store_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(job_store_error)
    }

    async fn count_generation_batches(
        &self,
        query: GenerationBatchHistoryQuery,
    ) -> JobResult<usize> {
        let connection = self.connection.lock().map_err(job_store_error)?;
        let sql = format!(
            "{GENERATION_BATCHES_CTE}\n\
             SELECT COUNT(*) FROM generation_batches \
             WHERE (?1 IS NULL OR batch_status = ?1)"
        );
        let count = connection
            .query_row(
                &sql,
                params![query.status.map(generation_batch_history_status_as_str)],
                |row| row.get::<_, i64>(0),
            )
            .map_err(job_store_error)?;
        usize::try_from(count).map_err(job_store_error)
    }

    async fn list_run_history_by_batch(&self, batch_id: &str) -> JobResult<Vec<RunHistoryRecord>> {
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
                    request_index,
                    expected_samples,
                    submitted_payload_ref,
                    prepared_payload_ref,
                    title,
                    last_error,
                    created_at_ms,
                    updated_at_ms,
                    completed_at_ms,
                    recoverable
                FROM run_history
                WHERE run_kind = 'generation' AND batch_id = ?1
                ORDER BY COALESCE(request_index, 2147483647), created_at_ms, run_id
                ",
            )
            .map_err(job_store_error)?;
        let rows = statement
            .query_map(params![batch_id], run_history_from_row)
            .map_err(job_store_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(job_store_error)
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

    async fn delete_generation_batches(&self, batch_ids: &[String]) -> JobResult<usize> {
        if batch_ids.is_empty() {
            return Ok(0);
        }
        let mut connection = self.connection.lock().map_err(job_store_error)?;
        let transaction = connection.transaction().map_err(job_store_error)?;
        let mut deleted = 0;
        for batch_id in batch_ids {
            deleted += transaction
                .execute(
                    "DELETE FROM run_history WHERE run_kind = 'generation' AND batch_id = ?1",
                    params![batch_id],
                )
                .map_err(job_store_error)?;
        }
        transaction.commit().map_err(job_store_error)?;
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
                        sample_index,
                        artifact_id,
                        item_id,
                        resource_id,
                        variant_id,
                        asset_role,
                        variant_kind
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ",
                    params![
                        output.run_id,
                        output.sample_index.map(i64::from),
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
                        sample_index,
                        artifact_id,
                        item_id,
                        resource_id,
                        variant_id,
                        asset_role,
                        variant_kind
                    FROM run_outputs
                    WHERE run_id = ?1
                    ORDER BY
                        COALESCE(sample_index, 2147483647) ASC,
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
                        sample_index: row
                            .get::<_, Option<i64>>(1)?
                            .map(|value| u32::try_from(value).unwrap_or(0)),
                        artifact_id: row.get(2)?,
                        item_id: row.get(3)?,
                        resource_id: row.get(4)?,
                        variant_id: row.get(5)?,
                        asset_role: row.get(6)?,
                        variant_kind: row.get(7)?,
                    })
                })
                .map_err(job_store_error)?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(job_store_error)?
        };
        Ok(outputs)
    }

    async fn delete_run_outputs_by_item_ids(&self, item_ids: &[String]) -> JobResult<usize> {
        if item_ids.is_empty() {
            return Ok(0);
        }

        let mut deleted = 0;
        {
            let mut connection = self.connection.lock().map_err(job_store_error)?;
            let transaction = connection.transaction().map_err(job_store_error)?;
            for item_id in item_ids {
                deleted += transaction
                    .execute(
                        "DELETE FROM run_outputs WHERE item_id = ?1",
                        params![item_id],
                    )
                    .map_err(job_store_error)?;
            }
            transaction.commit().map_err(job_store_error)?;
        }
        Ok(deleted)
    }
}

pub(super) fn upsert_run_history_row(
    connection: &rusqlite::Connection,
    record: &RunHistoryRecord,
) -> rusqlite::Result<()> {
    let created_at_ms = i64::try_from(record.created_at_ms).unwrap_or(i64::MAX);
    let updated_at_ms = i64::try_from(record.updated_at_ms).unwrap_or(i64::MAX);
    let completed_at_ms = record
        .completed_at_ms
        .map(|value| i64::try_from(value).unwrap_or(i64::MAX));
    connection.execute(
        r"
        INSERT INTO run_history(
            run_id, run_kind, run_status, batch_id, job_id, origin_run_id,
            request_index, expected_samples, submitted_payload_ref,
            prepared_payload_ref, title, last_error, created_at_ms,
            updated_at_ms, completed_at_ms, recoverable
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ON CONFLICT(run_id) DO UPDATE SET
            run_kind = excluded.run_kind,
            run_status = excluded.run_status,
            batch_id = excluded.batch_id,
            job_id = excluded.job_id,
            origin_run_id = excluded.origin_run_id,
            request_index = excluded.request_index,
            expected_samples = excluded.expected_samples,
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
            record.run_id.as_str(),
            run_history_kind_as_str(record.kind),
            run_history_status_as_str(record.status),
            record.batch_id.as_deref(),
            record.job_id.as_deref(),
            record.origin_run_id.as_deref(),
            record.request_index.map(i64::from),
            record.expected_samples.map(i64::from),
            record.submitted_payload_ref.as_deref(),
            record.prepared_payload_ref.as_deref(),
            record.title.as_deref(),
            record.last_error.as_deref(),
            created_at_ms,
            updated_at_ms,
            completed_at_ms,
            i64::from(record.recoverable),
        ],
    )?;
    Ok(())
}

const GENERATION_BATCHES_CTE: &str = r"
WITH generation_batches AS (
    SELECT
        history.batch_id AS batch_id,
        CASE
            WHEN SUM(history.run_status = 'paused') > 0 THEN 'paused'
            WHEN SUM(history.run_status = 'running') > 0 THEN 'running'
            WHEN SUM(history.run_status = 'preparing') > 0 THEN 'preparing'
            WHEN SUM(history.run_status = 'waiting') > 0 THEN 'waiting'
            WHEN SUM(history.run_status = 'queued') > 0 THEN 'queued'
            WHEN SUM(history.run_status = 'succeeded') = COUNT(*) THEN 'succeeded'
            WHEN SUM(history.run_status = 'succeeded') > 0 THEN 'partially_succeeded'
            WHEN SUM(history.run_status = 'failed') > 0 THEN 'failed'
            ELSE 'stopped'
        END AS batch_status,
        (
            SELECT first.title
            FROM run_history AS first
            WHERE first.run_kind = 'generation' AND first.batch_id = history.batch_id
            ORDER BY COALESCE(first.request_index, 2147483647), first.created_at_ms, first.run_id
            LIMIT 1
        ) AS title,
        (
            SELECT failed.last_error
            FROM run_history AS failed
            WHERE failed.run_kind = 'generation'
                AND failed.batch_id = history.batch_id
                AND failed.last_error IS NOT NULL
            ORDER BY failed.updated_at_ms DESC, failed.run_id
            LIMIT 1
        ) AS last_error,
        MIN(history.created_at_ms) AS created_at_ms,
        MAX(history.updated_at_ms) AS updated_at_ms,
        CASE
            WHEN SUM(history.run_status IN ('succeeded', 'failed', 'skipped', 'stopped')) = COUNT(*)
            THEN MAX(history.completed_at_ms)
            ELSE NULL
        END AS completed_at_ms,
        COUNT(*) AS request_count,
        SUM(history.run_status IN ('succeeded', 'failed', 'skipped', 'stopped'))
            AS completed_request_count,
        SUM(
            CASE
                WHEN COALESCE(history.expected_samples, 1) < 1 THEN 1
                ELSE COALESCE(history.expected_samples, 1)
            END
        ) AS expected_sample_count
    FROM run_history AS history
    WHERE history.run_kind = 'generation' AND history.batch_id IS NOT NULL
    GROUP BY history.batch_id
)
";

fn generation_batch_history_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<GenerationBatchHistoryRecord> {
    let status = generation_batch_history_status_from_str(&row.get::<_, String>(1)?)
        .map_err(super::scalars::to_sql_decode_error)?;
    Ok(GenerationBatchHistoryRecord {
        batch_id: row.get(0)?,
        status,
        title: row.get(2)?,
        last_error: row.get(3)?,
        created_at_ms: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
        updated_at_ms: u64::try_from(row.get::<_, i64>(5)?).unwrap_or(0),
        completed_at_ms: row
            .get::<_, Option<i64>>(6)?
            .map(|value| u64::try_from(value).unwrap_or(0)),
        request_count: usize::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
        completed_request_count: usize::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
        expected_sample_count: u32::try_from(row.get::<_, i64>(9)?).unwrap_or(0),
    })
}
