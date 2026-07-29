use async_trait::async_trait;

use crate::{
    GenerationBatchHistoryQuery, GenerationBatchHistoryRecord, JobBatch, JobEvent,
    JobQueueSnapshot, JobResult, RunHistoryQuery, RunHistoryRecord, RunOutputRecord,
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

    async fn upsert_run_history_batch(&self, records: Vec<RunHistoryRecord>) -> JobResult<()>;

    async fn get_run_history(&self, run_id: &str) -> JobResult<Option<RunHistoryRecord>>;

    async fn query_run_history(&self, query: RunHistoryQuery) -> JobResult<Vec<RunHistoryRecord>>;

    async fn count_run_history(&self, query: RunHistoryQuery) -> JobResult<usize>;

    async fn run_history_batch_exists(&self, batch_id: &str) -> JobResult<bool>;

    async fn query_generation_batches(
        &self,
        query: GenerationBatchHistoryQuery,
    ) -> JobResult<Vec<GenerationBatchHistoryRecord>>;

    async fn count_generation_batches(
        &self,
        query: GenerationBatchHistoryQuery,
    ) -> JobResult<usize>;

    async fn list_run_history_by_batch(&self, batch_id: &str) -> JobResult<Vec<RunHistoryRecord>>;

    async fn delete_run_history_items(&self, run_ids: &[String]) -> JobResult<usize>;

    async fn delete_generation_batches(&self, batch_ids: &[String]) -> JobResult<usize>;

    async fn upsert_run_output(&self, output: RunOutputRecord) -> JobResult<()>;

    async fn list_run_outputs(&self, run_id: &str) -> JobResult<Vec<RunOutputRecord>>;

    async fn mark_run_outputs_deleted_by_item_ids(&self, item_ids: &[String]) -> JobResult<usize>;
}
