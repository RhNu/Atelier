use super::{
    ActiveJobBatchSnapshot, BatchStatus, GenerationRequestStatusDto, GenerationStatusDto,
    QueueDelay, QueueDelayDto, QueueDirective, QueueDirectiveDto, RunHistoryRecord,
};

pub fn queue_directive_to_dto(value: QueueDirective) -> QueueDirectiveDto {
    match value {
        QueueDirective::StartJob(id) => QueueDirectiveDto::StartJob {
            job_id: id.as_str().to_owned(),
        },
        QueueDirective::Wait(delay) => QueueDirectiveDto::Wait {
            delay: queue_delay_to_dto(delay),
        },
        QueueDirective::Paused => QueueDirectiveDto::Paused,
        QueueDirective::Idle => QueueDirectiveDto::Idle,
    }
}

pub fn generation_status_to_dto(
    snapshot: Option<ActiveJobBatchSnapshot>,
    history: &[RunHistoryRecord],
    requested_job_id: Option<&str>,
) -> GenerationStatusDto {
    let Some(active) = snapshot else {
        return GenerationStatusDto::default();
    };
    let current_job_id = active.current_job.as_ref().map(|id| id.as_str().to_owned());
    let job_status = requested_job_id
        .or(current_job_id.as_deref())
        .and_then(|id| {
            active
                .batch
                .jobs
                .iter()
                .find(|job| job.job_id.as_str() == id)
        })
        .map(|job| job_status_as_str(job.status).to_owned());
    let requests = active
        .batch
        .jobs
        .iter()
        .enumerate()
        .map(|(index, job)| {
            let record = history
                .iter()
                .find(|record| record.job_id.as_deref() == Some(job.job_id.as_str()));
            GenerationRequestStatusDto {
                job_id: job.job_id.as_str().to_owned(),
                request_index: record
                    .and_then(|record| record.request_index)
                    .unwrap_or_else(|| u32::try_from(index).unwrap_or(u32::MAX)),
                expected_samples: record
                    .and_then(|record| record.expected_samples)
                    .unwrap_or(1)
                    .max(1),
                status: job_status_as_str(job.status).to_owned(),
            }
        })
        .collect();
    GenerationStatusDto {
        batch_id: Some(active.batch.batch_id.as_str().to_owned()),
        batch_status: Some(batch_status_as_str(active.batch.status).to_owned()),
        current_job_id,
        job_status,
        requests,
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

const fn job_status_as_str(value: atelier_jobs::JobStatus) -> &'static str {
    match value {
        atelier_jobs::JobStatus::Queued => "queued",
        atelier_jobs::JobStatus::Preparing => "preparing",
        atelier_jobs::JobStatus::Running => "running",
        atelier_jobs::JobStatus::WaitingRetry => "waiting_retry",
        atelier_jobs::JobStatus::Blocked => "blocked",
        atelier_jobs::JobStatus::Succeeded => "succeeded",
        atelier_jobs::JobStatus::Failed => "failed",
        atelier_jobs::JobStatus::Skipped => "skipped",
    }
}

fn queue_delay_to_dto(value: QueueDelay) -> QueueDelayDto {
    QueueDelayDto {
        min_ms: value.min.as_millis().try_into().unwrap_or(u64::MAX),
        max_ms: value.max.as_millis().try_into().unwrap_or(u64::MAX),
    }
}
