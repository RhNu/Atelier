use async_trait::async_trait;

use crate::{
    JobBatch, JobEvent, JobQueueSnapshot, JobResult, RunHistoryQuery, RunHistoryRecord,
    RunOutputRecord,
};

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn load_active_batch(&self) -> JobResult<Option<JobBatch>>;

    async fn save_batch(&self, batch: &JobBatch) -> JobResult<()>;
}

#[async_trait]
pub trait JobEventSink: Send + Sync {
    async fn publish(&self, event: JobEvent) -> JobResult<()>;
}

#[async_trait]
pub trait JobQueueRepository: Send + Sync {
    async fn load_queue_snapshot(&self) -> JobResult<Option<JobQueueSnapshot>>;

    async fn save_queue_snapshot(&self, snapshot: &JobQueueSnapshot) -> JobResult<()>;

    async fn clear_queue_snapshot(&self) -> JobResult<()>;
}

#[async_trait]
pub trait RunHistoryRepository: Send + Sync {
    async fn upsert_run_history(&self, record: RunHistoryRecord) -> JobResult<()>;

    async fn get_run_history(&self, run_id: &str) -> JobResult<Option<RunHistoryRecord>>;

    async fn query_run_history(&self, query: RunHistoryQuery) -> JobResult<Vec<RunHistoryRecord>>;

    async fn count_run_history(&self, query: RunHistoryQuery) -> JobResult<usize>;

    async fn run_history_batch_exists(&self, batch_id: &str) -> JobResult<bool>;

    async fn upsert_run_output(&self, output: RunOutputRecord) -> JobResult<()>;

    async fn list_run_outputs(&self, run_id: &str) -> JobResult<Vec<RunOutputRecord>>;
}
