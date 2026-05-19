//! Single-queue job state machine for NAI Atelier.

mod error;
mod model;
mod ports;
mod queue;

pub use error::{JobQueueError, JobQueueErrorKind, JobResult};
pub use model::{
    BatchId, BatchStatus, JobBatch, JobEvent, JobEventKind, JobFailureImpact, JobId, JobKind,
    JobPayloadRef, JobRecord, JobStatus, QueueCommand, QueueDelay, QueueDirective, RetryPolicy,
    SubmitJob,
};
pub use ports::{JobEventSink, JobRepository};
pub use queue::JobQueue;
