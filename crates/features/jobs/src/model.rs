use std::time::Duration;

use nai_atelier_foundation::{NovelAiError, NovelAiErrorKind};

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

impl JobFailureImpact {
    #[must_use]
    pub fn from_novelai_error(error: &NovelAiError) -> Self {
        Self::from_novelai_error_with_policy(error, RetryPolicy::default())
    }

    #[must_use]
    pub fn from_novelai_error_with_policy(error: &NovelAiError, policy: RetryPolicy) -> Self {
        match error.kind {
            NovelAiErrorKind::RateLimited => {
                let delay = error
                    .retry_after
                    .map_or_else(|| policy.rate_limit_fallback, QueueDelay::fixed);
                Self::RetryAfter(delay)
            }
            NovelAiErrorKind::InvalidRequest => Self::FailCurrentAndContinue,
            NovelAiErrorKind::Credential
            | NovelAiErrorKind::Authentication
            | NovelAiErrorKind::InsufficientCredit
            | NovelAiErrorKind::RequestConflict
            | NovelAiErrorKind::ServiceUnavailable
            | NovelAiErrorKind::Transport
            | NovelAiErrorKind::Decode
            | NovelAiErrorKind::Metadata
            | NovelAiErrorKind::UnknownApi => Self::PauseAndRetryCurrent,
        }
    }
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
