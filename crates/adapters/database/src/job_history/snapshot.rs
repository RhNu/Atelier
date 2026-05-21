use super::{
    ActiveJobBatchSnapshot, BatchId, Deserialize, Duration, JobBatch, JobId, JobPayloadRef,
    JobQueueSnapshot, JobRecord, JobResult, QueueDelay, RetryPolicy, Serialize,
    batch_status_as_str, batch_status_from_str, duration_to_ms, job_kind_as_str, job_kind_from_str,
    job_status_as_str, job_status_from_str,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct JobQueueSnapshotDto {
    active_batch: Option<ActiveJobBatchSnapshotDto>,
    retry_policy: RetryPolicyDto,
}

impl JobQueueSnapshotDto {
    pub(super) fn from_domain(value: &JobQueueSnapshot) -> Self {
        Self {
            active_batch: value
                .active_batch
                .as_ref()
                .map(ActiveJobBatchSnapshotDto::from_domain),
            retry_policy: RetryPolicyDto::from_domain(value.retry_policy),
        }
    }

    pub(super) fn into_domain(self) -> JobResult<JobQueueSnapshot> {
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
pub(super) struct ActiveJobBatchSnapshotDto {
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
pub(super) struct JobBatchDto {
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
pub(super) struct JobRecordDto {
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
pub(super) struct RetryPolicyDto {
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
pub(super) struct QueueDelayDto {
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
