use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BatchId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobPayloadRef(String);

macro_rules! opaque_id {
    ($type_name:ident) => {
        impl $type_name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

opaque_id!(BatchId);
opaque_id!(JobId);
opaque_id!(JobPayloadRef);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JobKind {
    GenerateImage,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BatchStatus {
    Running,
    Waiting,
    Paused,
    Stopping,
    Succeeded,
    Stopped,
}

impl BatchStatus {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Stopped)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Preparing,
    Running,
    WaitingRetry,
    Blocked,
    Succeeded,
    Failed,
    Skipped,
}

impl JobStatus {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Skipped)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct QueueDelay {
    pub min: Duration,
    pub max: Duration,
}

impl QueueDelay {
    #[must_use]
    pub const fn fixed(duration: Duration) -> Self {
        Self {
            min: duration,
            max: duration,
        }
    }

    #[must_use]
    pub const fn range(min: Duration, max: Duration) -> Self {
        Self { min, max }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub task_interval: QueueDelay,
    pub rate_limit_fallback: QueueDelay,
    pub max_rate_limit_retries: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            task_interval: QueueDelay::fixed(Duration::from_secs(3)),
            rate_limit_fallback: QueueDelay::range(
                Duration::from_secs(20),
                Duration::from_secs(30),
            ),
            max_rate_limit_retries: 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueDirective {
    StartJob(JobId),
    Wait(QueueDelay),
    Paused,
    Idle,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QueueCommand {
    StartNext,
    DelayElapsed,
    Pause,
    Resume,
    Stop,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum JobFailureImpact {
    RetryAfter(QueueDelay),
    FailCurrentAndContinue,
    PauseAndRetryCurrent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmitJob {
    pub job_id: JobId,
    pub kind: JobKind,
    pub payload_ref: JobPayloadRef,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRecord {
    pub job_id: JobId,
    pub kind: JobKind,
    pub payload_ref: JobPayloadRef,
    pub prepared_payload_ref: Option<JobPayloadRef>,
    pub status: JobStatus,
    pub retry_attempts: u32,
}

impl From<SubmitJob> for JobRecord {
    fn from(job: SubmitJob) -> Self {
        Self {
            job_id: job.job_id,
            kind: job.kind,
            payload_ref: job.payload_ref,
            prepared_payload_ref: None,
            status: JobStatus::Queued,
            retry_attempts: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobBatch {
    pub batch_id: BatchId,
    pub status: BatchStatus,
    pub jobs: Vec<JobRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobQueueSnapshot {
    pub active_batch: Option<ActiveJobBatchSnapshot>,
    pub retry_policy: RetryPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveJobBatchSnapshot {
    pub batch: JobBatch,
    pub current_job: Option<JobId>,
    pub pending_delay: Option<QueueDelay>,
    pub paused_delay: Option<QueueDelay>,
    pub pause_after_current: bool,
    pub stop_after_current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobEvent {
    pub batch_id: BatchId,
    pub job_id: Option<JobId>,
    pub kind: JobEventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobEventKind {
    BatchSubmitted,
    JobStarted,
    JobSucceeded,
    JobFailed,
    BatchPaused,
    BatchStopped,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RunHistoryKind {
    Generation,
    Director,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RunHistoryStatus {
    Queued,
    Preparing,
    Running,
    Waiting,
    Paused,
    Succeeded,
    Failed,
    Skipped,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunHistoryRecord {
    pub run_id: String,
    pub kind: RunHistoryKind,
    pub status: RunHistoryStatus,
    pub batch_id: Option<String>,
    pub job_id: Option<String>,
    pub origin_run_id: Option<String>,
    pub submitted_payload_ref: Option<String>,
    pub prepared_payload_ref: Option<String>,
    pub title: Option<String>,
    pub last_error: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub recoverable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutputRecord {
    pub run_id: String,
    pub artifact_id: String,
    pub item_id: Option<String>,
    pub resource_id: String,
    pub variant_id: Option<String>,
    pub asset_role: String,
    pub variant_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunHistoryQuery {
    pub offset: usize,
    pub limit: usize,
    pub kind: Option<RunHistoryKind>,
    pub status: Option<RunHistoryStatus>,
}

impl Default for RunHistoryQuery {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            kind: None,
            status: None,
        }
    }
}
