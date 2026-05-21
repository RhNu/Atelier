#![allow(clippy::significant_drop_tightening)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use nai_atelier_jobs::{
    ActiveJobBatchSnapshot, BatchId, BatchStatus, JobBatch, JobId, JobKind, JobPayloadRef,
    JobQueueError, JobQueueRepository, JobQueueSnapshot, JobRecord, JobResult, JobStatus,
    QueueDelay, RetryPolicy, RunHistoryKind, RunHistoryQuery, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus, RunOutputRecord,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::DatabaseConnection;
use crate::codec::{decode_json, encode_json};

#[derive(Clone, Debug)]
pub struct DatabaseJobQueueRepository {
    connection: DatabaseConnection,
}

impl DatabaseJobQueueRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

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

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JobQueueSnapshotDto {
    active_batch: Option<ActiveJobBatchSnapshotDto>,
    retry_policy: RetryPolicyDto,
}

impl JobQueueSnapshotDto {
    fn from_domain(value: &JobQueueSnapshot) -> Self {
        Self {
            active_batch: value
                .active_batch
                .as_ref()
                .map(ActiveJobBatchSnapshotDto::from_domain),
            retry_policy: RetryPolicyDto::from_domain(value.retry_policy),
        }
    }

    fn into_domain(self) -> JobResult<JobQueueSnapshot> {
        Ok(JobQueueSnapshot {
            active_batch: self
                .active_batch
                .map(ActiveJobBatchSnapshotDto::into_domain)
                .transpose()?,
            retry_policy: self.retry_policy.into_domain(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ActiveJobBatchSnapshotDto {
    batch: JobBatchDto,
    current_job: Option<String>,
    pending_delay: Option<QueueDelayDto>,
    paused_delay: Option<QueueDelayDto>,
    pause_after_current: bool,
    stop_after_current: bool,
}

impl ActiveJobBatchSnapshotDto {
    fn from_domain(value: &ActiveJobBatchSnapshot) -> Self {
        Self {
            batch: JobBatchDto::from_domain(&value.batch),
            current_job: value.current_job.as_ref().map(|id| id.as_str().to_owned()),
            pending_delay: value.pending_delay.map(QueueDelayDto::from_domain),
            paused_delay: value.paused_delay.map(QueueDelayDto::from_domain),
            pause_after_current: value.pause_after_current,
            stop_after_current: value.stop_after_current,
        }
    }

    fn into_domain(self) -> JobResult<ActiveJobBatchSnapshot> {
        Ok(ActiveJobBatchSnapshot {
            batch: self.batch.into_domain()?,
            current_job: self.current_job.map(JobId::new),
            pending_delay: self.pending_delay.map(QueueDelayDto::into_domain),
            paused_delay: self.paused_delay.map(QueueDelayDto::into_domain),
            pause_after_current: self.pause_after_current,
            stop_after_current: self.stop_after_current,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JobBatchDto {
    batch_id: String,
    status: String,
    jobs: Vec<JobRecordDto>,
}

impl JobBatchDto {
    fn from_domain(value: &JobBatch) -> Self {
        Self {
            batch_id: value.batch_id.as_str().to_owned(),
            status: batch_status_as_str(value.status).to_owned(),
            jobs: value.jobs.iter().map(JobRecordDto::from_domain).collect(),
        }
    }

    fn into_domain(self) -> JobResult<JobBatch> {
        Ok(JobBatch {
            batch_id: BatchId::new(self.batch_id),
            status: batch_status_from_str(&self.status)?,
            jobs: self
                .jobs
                .into_iter()
                .map(JobRecordDto::into_domain)
                .collect::<JobResult<Vec<_>>>()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JobRecordDto {
    job_id: String,
    kind: String,
    payload_ref: String,
    prepared_payload_ref: Option<String>,
    status: String,
    retry_attempts: u32,
}

impl JobRecordDto {
    fn from_domain(value: &JobRecord) -> Self {
        Self {
            job_id: value.job_id.as_str().to_owned(),
            kind: job_kind_as_str(value.kind).to_owned(),
            payload_ref: value.payload_ref.as_str().to_owned(),
            prepared_payload_ref: value
                .prepared_payload_ref
                .as_ref()
                .map(|id| id.as_str().to_owned()),
            status: job_status_as_str(value.status).to_owned(),
            retry_attempts: value.retry_attempts,
        }
    }

    fn into_domain(self) -> JobResult<JobRecord> {
        Ok(JobRecord {
            job_id: JobId::new(self.job_id),
            kind: job_kind_from_str(&self.kind)?,
            payload_ref: JobPayloadRef::new(self.payload_ref),
            prepared_payload_ref: self.prepared_payload_ref.map(JobPayloadRef::new),
            status: job_status_from_str(&self.status)?,
            retry_attempts: self.retry_attempts,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RetryPolicyDto {
    task_interval: QueueDelayDto,
    rate_limit_fallback: QueueDelayDto,
    max_rate_limit_retries: u32,
}

impl RetryPolicyDto {
    fn from_domain(value: RetryPolicy) -> Self {
        Self {
            task_interval: QueueDelayDto::from_domain(value.task_interval),
            rate_limit_fallback: QueueDelayDto::from_domain(value.rate_limit_fallback),
            max_rate_limit_retries: value.max_rate_limit_retries,
        }
    }

    const fn into_domain(self) -> RetryPolicy {
        RetryPolicy {
            task_interval: self.task_interval.into_domain(),
            rate_limit_fallback: self.rate_limit_fallback.into_domain(),
            max_rate_limit_retries: self.max_rate_limit_retries,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct QueueDelayDto {
    min_ms: u64,
    max_ms: u64,
}

impl QueueDelayDto {
    fn from_domain(value: QueueDelay) -> Self {
        Self {
            min_ms: duration_to_ms(value.min),
            max_ms: duration_to_ms(value.max),
        }
    }

    const fn into_domain(self) -> QueueDelay {
        QueueDelay::range(
            Duration::from_millis(self.min_ms),
            Duration::from_millis(self.max_ms),
        )
    }
}

fn run_history_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunHistoryRecord> {
    Ok(RunHistoryRecord {
        run_id: row.get(0)?,
        kind: run_history_kind_from_str(&row.get::<_, String>(1)?).map_err(to_sql_decode_error)?,
        status: run_history_status_from_str(&row.get::<_, String>(2)?)
            .map_err(to_sql_decode_error)?,
        batch_id: row.get(3)?,
        job_id: row.get(4)?,
        origin_run_id: row.get(5)?,
        submitted_payload_ref: row.get(6)?,
        prepared_payload_ref: row.get(7)?,
        title: row.get(8)?,
        last_error: row.get(9)?,
        created_at_ms: u64::try_from(row.get::<_, i64>(10)?).unwrap_or(0),
        updated_at_ms: u64::try_from(row.get::<_, i64>(11)?).unwrap_or(0),
        completed_at_ms: row
            .get::<_, Option<i64>>(12)?
            .map(|value| u64::try_from(value).unwrap_or(0)),
        recoverable: row.get::<_, i64>(13)? != 0,
    })
}

fn duration_to_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn now_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

const fn job_kind_as_str(value: JobKind) -> &'static str {
    match value {
        JobKind::GenerateImage => "generate_image",
    }
}

fn job_kind_from_str(value: &str) -> JobResult<JobKind> {
    match value {
        "generate_image" => Ok(JobKind::GenerateImage),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown job kind `{value}`"
        ))),
    }
}

const fn batch_status_as_str(value: BatchStatus) -> &'static str {
    match value {
        BatchStatus::Running => "running",
        BatchStatus::Waiting => "waiting",
        BatchStatus::Paused => "paused",
        BatchStatus::Stopping => "stopping",
        BatchStatus::Succeeded => "succeeded",
        BatchStatus::Stopped => "stopped",
    }
}

fn batch_status_from_str(value: &str) -> JobResult<BatchStatus> {
    match value {
        "running" => Ok(BatchStatus::Running),
        "waiting" => Ok(BatchStatus::Waiting),
        "paused" => Ok(BatchStatus::Paused),
        "stopping" => Ok(BatchStatus::Stopping),
        "succeeded" => Ok(BatchStatus::Succeeded),
        "stopped" => Ok(BatchStatus::Stopped),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown batch status `{value}`"
        ))),
    }
}

const fn job_status_as_str(value: JobStatus) -> &'static str {
    match value {
        JobStatus::Queued => "queued",
        JobStatus::Preparing => "preparing",
        JobStatus::Running => "running",
        JobStatus::WaitingRetry => "waiting_retry",
        JobStatus::Blocked => "blocked",
        JobStatus::Succeeded => "succeeded",
        JobStatus::Failed => "failed",
        JobStatus::Skipped => "skipped",
    }
}

fn job_status_from_str(value: &str) -> JobResult<JobStatus> {
    match value {
        "queued" => Ok(JobStatus::Queued),
        "preparing" => Ok(JobStatus::Preparing),
        "running" => Ok(JobStatus::Running),
        "waiting_retry" => Ok(JobStatus::WaitingRetry),
        "blocked" => Ok(JobStatus::Blocked),
        "succeeded" => Ok(JobStatus::Succeeded),
        "failed" => Ok(JobStatus::Failed),
        "skipped" => Ok(JobStatus::Skipped),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown job status `{value}`"
        ))),
    }
}

const fn run_history_kind_as_str(value: RunHistoryKind) -> &'static str {
    match value {
        RunHistoryKind::Generation => "generation",
        RunHistoryKind::Director => "director",
    }
}

fn run_history_kind_from_str(value: &str) -> JobResult<RunHistoryKind> {
    match value {
        "generation" => Ok(RunHistoryKind::Generation),
        "director" => Ok(RunHistoryKind::Director),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown run kind `{value}`"
        ))),
    }
}

const fn run_history_status_as_str(value: RunHistoryStatus) -> &'static str {
    match value {
        RunHistoryStatus::Queued => "queued",
        RunHistoryStatus::Preparing => "preparing",
        RunHistoryStatus::Running => "running",
        RunHistoryStatus::Waiting => "waiting",
        RunHistoryStatus::Paused => "paused",
        RunHistoryStatus::Succeeded => "succeeded",
        RunHistoryStatus::Failed => "failed",
        RunHistoryStatus::Skipped => "skipped",
        RunHistoryStatus::Stopped => "stopped",
    }
}

fn run_history_status_from_str(value: &str) -> JobResult<RunHistoryStatus> {
    match value {
        "queued" => Ok(RunHistoryStatus::Queued),
        "preparing" => Ok(RunHistoryStatus::Preparing),
        "running" => Ok(RunHistoryStatus::Running),
        "waiting" => Ok(RunHistoryStatus::Waiting),
        "paused" => Ok(RunHistoryStatus::Paused),
        "succeeded" => Ok(RunHistoryStatus::Succeeded),
        "failed" => Ok(RunHistoryStatus::Failed),
        "skipped" => Ok(RunHistoryStatus::Skipped),
        "stopped" => Ok(RunHistoryStatus::Stopped),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown run status `{value}`"
        ))),
    }
}

fn job_store_error(error: impl std::fmt::Display) -> JobQueueError {
    JobQueueError::invalid_state(error.to_string())
}

fn to_sql_decode_error(error: JobQueueError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
