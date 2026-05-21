//! Single-queue job state machine for NAI Atelier.

mod error;
mod model;
mod ports;
mod queue;

pub use error::{JobQueueError, JobQueueErrorKind, JobResult};
pub use model::{
    ActiveJobBatchSnapshot, BatchId, BatchStatus, JobBatch, JobEvent, JobEventKind,
    JobFailureImpact, JobId, JobKind, JobPayloadRef, JobQueueSnapshot, JobRecord, JobStatus,
    QueueCommand, QueueDelay, QueueDirective, RetryPolicy, RunHistoryKind, RunHistoryQuery,
    RunHistoryRecord, RunHistoryStatus, RunOutputRecord, SubmitJob,
};
pub use ports::{JobEventSink, JobQueueRepository, JobRepository, RunHistoryRepository};
pub use queue::JobQueue;
