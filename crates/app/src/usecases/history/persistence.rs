use std::time::{SystemTime, UNIX_EPOCH};

use atelier_adapter_database::DatabaseRunHistoryRepository;
use atelier_jobs::{
    BatchStatus, JobKind, JobQueueSnapshot, JobRecord, JobStatus, RunHistoryKind, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus,
};

use crate::{AppError, AppResult};

pub(super) async fn ensure_generation_history_target_is_new(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job_id: &str,
) -> AppResult<()> {
    if repository
        .get_run_history(job_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?
        .is_some()
    {
        return Err(AppError::new(
            "invalid_request",
            "generation job_id already exists in run history",
        ));
    }
    if repository
        .run_history_batch_exists(batch_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?
    {
        return Err(AppError::new(
            "invalid_request",
            "generation batch_id already exists in run history",
        ));
    }
    Ok(())
}

pub async fn upsert_generation_history_record(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job_id: &str,
    update: GenerationHistoryUpdate,
) -> AppResult<RunHistoryRecord> {
    let now = unix_timestamp_ms();
    let existing = repository
        .get_run_history(job_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    let created_at_ms = existing.as_ref().map_or(now, |record| record.created_at_ms);
    let record = RunHistoryRecord {
        run_id: job_id.to_owned(),
        kind: RunHistoryKind::Generation,
        status: update.status,
        batch_id: Some(batch_id.to_owned()),
        job_id: Some(job_id.to_owned()),
        origin_run_id: update.origin_run_id.or_else(|| {
            existing
                .as_ref()
                .and_then(|record| record.origin_run_id.clone())
        }),
        request_index: update
            .position
            .map(|value| value.request_index)
            .or_else(|| existing.as_ref().and_then(|record| record.request_index)),
        expected_samples: update
            .position
            .map(|value| value.expected_samples.max(1))
            .or_else(|| existing.as_ref().and_then(|record| record.expected_samples)),
        submitted_payload_ref: Some(format!("generation-submitted:{job_id}")),
        prepared_payload_ref: existing
            .as_ref()
            .and_then(|record| record.prepared_payload_ref.clone()),
        title: update
            .title
            .or_else(|| existing.as_ref().and_then(|record| record.title.clone())),
        last_error: update.last_error,
        created_at_ms,
        updated_at_ms: now,
        completed_at_ms: status_is_terminal(update.status).then_some(now),
        recoverable: update.status == RunHistoryStatus::Paused,
    };
    repository
        .upsert_run_history(record.clone())
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    Ok(record)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenerationHistoryPosition {
    pub request_index: u32,
    pub expected_samples: u32,
}

pub struct GenerationHistoryUpdate {
    pub status: RunHistoryStatus,
    pub title: Option<String>,
    pub origin_run_id: Option<String>,
    pub last_error: Option<String>,
    pub position: Option<GenerationHistoryPosition>,
}

pub async fn generation_history_records_from_queue_snapshot(
    repository: &DatabaseRunHistoryRepository,
    snapshot: &JobQueueSnapshot,
) -> AppResult<Vec<RunHistoryRecord>> {
    let Some(active_batch) = &snapshot.active_batch else {
        return Ok(Vec::new());
    };
    let paused_job_id = if active_batch.batch.status == BatchStatus::Paused {
        active_batch.current_job.as_ref().or_else(|| {
            active_batch
                .batch
                .jobs
                .iter()
                .find(|job| !job.status.is_terminal())
                .map(|job| &job.job_id)
        })
    } else {
        None
    };
    let mut records = Vec::new();
    for (request_index, job) in active_batch.batch.jobs.iter().enumerate() {
        if job.kind != JobKind::GenerateImage {
            continue;
        }
        let status = if paused_job_id.is_some_and(|job_id| job_id == &job.job_id) {
            RunHistoryStatus::Paused
        } else {
            run_history_status_from_job_status(job.status)
        };
        records.push(
            build_generation_history_from_job(
                repository,
                active_batch.batch.batch_id.as_str(),
                job,
                status,
                u32::try_from(request_index).unwrap_or(u32::MAX),
            )
            .await?,
        );
    }
    Ok(records)
}

async fn build_generation_history_from_job(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job: &JobRecord,
    status: RunHistoryStatus,
    request_index: u32,
) -> AppResult<RunHistoryRecord> {
    let now = unix_timestamp_ms();
    let existing = repository
        .get_run_history(job.job_id.as_str())
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    let record = RunHistoryRecord {
        run_id: job.job_id.as_str().to_owned(),
        kind: RunHistoryKind::Generation,
        status,
        batch_id: Some(batch_id.to_owned()),
        job_id: Some(job.job_id.as_str().to_owned()),
        origin_run_id: existing
            .as_ref()
            .and_then(|record| record.origin_run_id.clone()),
        request_index: existing
            .as_ref()
            .and_then(|record| record.request_index)
            .or(Some(request_index)),
        expected_samples: existing
            .as_ref()
            .and_then(|record| record.expected_samples)
            .or(Some(1)),
        submitted_payload_ref: Some(job.payload_ref.as_str().to_owned()),
        prepared_payload_ref: job
            .prepared_payload_ref
            .as_ref()
            .map(|id| id.as_str().to_owned())
            .or_else(|| {
                existing
                    .as_ref()
                    .and_then(|record| record.prepared_payload_ref.clone())
            }),
        title: existing.as_ref().and_then(|record| record.title.clone()),
        last_error: existing
            .as_ref()
            .and_then(|record| record.last_error.clone()),
        created_at_ms: existing.as_ref().map_or(now, |record| record.created_at_ms),
        updated_at_ms: now,
        completed_at_ms: if status_is_terminal(status) {
            existing
                .as_ref()
                .and_then(|record| record.completed_at_ms)
                .or(Some(now))
        } else {
            None
        },
        recoverable: status == RunHistoryStatus::Paused,
    };
    Ok(record)
}

const fn run_history_status_from_job_status(status: JobStatus) -> RunHistoryStatus {
    match status {
        JobStatus::Queued => RunHistoryStatus::Queued,
        JobStatus::Preparing => RunHistoryStatus::Preparing,
        JobStatus::Running => RunHistoryStatus::Running,
        JobStatus::WaitingRetry => RunHistoryStatus::Waiting,
        JobStatus::Blocked => RunHistoryStatus::Paused,
        JobStatus::Succeeded => RunHistoryStatus::Succeeded,
        JobStatus::Failed => RunHistoryStatus::Failed,
        JobStatus::Skipped => RunHistoryStatus::Skipped,
    }
}

pub(super) const fn status_is_terminal(status: RunHistoryStatus) -> bool {
    matches!(
        status,
        RunHistoryStatus::Succeeded
            | RunHistoryStatus::Failed
            | RunHistoryStatus::Skipped
            | RunHistoryStatus::Stopped
    )
}

fn unix_timestamp_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
