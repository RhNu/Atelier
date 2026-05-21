use super::{
    BatchStatus, Duration, JobKind, JobQueueError, JobResult, JobStatus, RunHistoryKind,
    RunHistoryRecord, RunHistoryStatus, SystemTime, UNIX_EPOCH,
};

pub(super) fn run_history_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunHistoryRecord> {
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

pub(super) fn duration_to_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

pub(super) fn now_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}

pub(super) const fn job_kind_as_str(value: JobKind) -> &'static str {
    match value {
        JobKind::GenerateImage => "generate_image",
    }
}

pub(super) fn job_kind_from_str(value: &str) -> JobResult<JobKind> {
    match value {
        "generate_image" => Ok(JobKind::GenerateImage),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown job kind `{value}`"
        ))),
    }
}

pub(super) const fn batch_status_as_str(value: BatchStatus) -> &'static str {
    match value {
        BatchStatus::Running => "running",
        BatchStatus::Waiting => "waiting",
        BatchStatus::Paused => "paused",
        BatchStatus::Stopping => "stopping",
        BatchStatus::Succeeded => "succeeded",
        BatchStatus::Stopped => "stopped",
    }
}

pub(super) fn batch_status_from_str(value: &str) -> JobResult<BatchStatus> {
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

pub(super) const fn job_status_as_str(value: JobStatus) -> &'static str {
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

pub(super) fn job_status_from_str(value: &str) -> JobResult<JobStatus> {
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

pub(super) const fn run_history_kind_as_str(value: RunHistoryKind) -> &'static str {
    match value {
        RunHistoryKind::Generation => "generation",
        RunHistoryKind::Director => "director",
    }
}

pub(super) fn run_history_kind_from_str(value: &str) -> JobResult<RunHistoryKind> {
    match value {
        "generation" => Ok(RunHistoryKind::Generation),
        "director" => Ok(RunHistoryKind::Director),
        _ => Err(JobQueueError::invalid_state(format!(
            "unknown run kind `{value}`"
        ))),
    }
}

pub(super) const fn run_history_status_as_str(value: RunHistoryStatus) -> &'static str {
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

pub(super) fn run_history_status_from_str(value: &str) -> JobResult<RunHistoryStatus> {
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

pub(super) fn job_store_error(error: impl std::fmt::Display) -> JobQueueError {
    JobQueueError::invalid_state(error.to_string())
}

pub(super) fn to_sql_decode_error(error: JobQueueError) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
