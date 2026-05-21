use super::{
    BatchStatus, GenerationStatusDto, QueueDelay, QueueDelayDto, QueueDirective, QueueDirectiveDto,
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
    batch: Option<BatchStatus>,
    job: Option<nai_atelier_jobs::JobStatus>,
) -> GenerationStatusDto {
    GenerationStatusDto {
        batch_status: batch.map(|value| batch_status_as_str(value).to_owned()),
        job_status: job.map(|value| job_status_as_str(value).to_owned()),
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

const fn job_status_as_str(value: nai_atelier_jobs::JobStatus) -> &'static str {
    match value {
        nai_atelier_jobs::JobStatus::Queued => "queued",
        nai_atelier_jobs::JobStatus::Preparing => "preparing",
        nai_atelier_jobs::JobStatus::Running => "running",
        nai_atelier_jobs::JobStatus::WaitingRetry => "waiting_retry",
        nai_atelier_jobs::JobStatus::Blocked => "blocked",
        nai_atelier_jobs::JobStatus::Succeeded => "succeeded",
        nai_atelier_jobs::JobStatus::Failed => "failed",
        nai_atelier_jobs::JobStatus::Skipped => "skipped",
    }
}

fn queue_delay_to_dto(value: QueueDelay) -> QueueDelayDto {
    QueueDelayDto {
        min_ms: value.min.as_millis().try_into().unwrap_or(u64::MAX),
        max_ms: value.max.as_millis().try_into().unwrap_or(u64::MAX),
    }
}
