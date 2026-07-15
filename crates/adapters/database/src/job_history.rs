#![allow(clippy::significant_drop_tightening)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use atelier_jobs::{
    ActiveJobBatchSnapshot, BatchId, BatchStatus, GenerationBatchHistoryQuery,
    GenerationBatchHistoryRecord, GenerationBatchHistoryStatus, JobBatch, JobId, JobKind,
    JobPayloadRef, JobQueueError, JobQueueRepository, JobQueueSnapshot, JobRecord, JobResult,
    JobStatus, QueueDelay, RetryPolicy, RunHistoryKind, RunHistoryQuery, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus, RunOutputRecord,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::DatabaseConnection;
use crate::codec::{decode_json, encode_json};

mod queue;
mod run_history;
mod scalars;
mod snapshot;

pub use queue::DatabaseJobQueueRepository;
pub use run_history::DatabaseRunHistoryRepository;
use scalars::{
    batch_status_as_str, batch_status_from_str, duration_to_ms,
    generation_batch_history_status_as_str, generation_batch_history_status_from_str,
    job_kind_as_str, job_kind_from_str, job_status_as_str, job_status_from_str, job_store_error,
    now_ms, run_history_from_row, run_history_kind_as_str, run_history_status_as_str,
};
use snapshot::JobQueueSnapshotDto;
