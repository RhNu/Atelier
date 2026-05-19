use async_trait::async_trait;

use crate::{JobBatch, JobEvent, JobResult};

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn load_active_batch(&self) -> JobResult<Option<JobBatch>>;

    async fn save_batch(&self, batch: &JobBatch) -> JobResult<()>;
}

#[async_trait]
pub trait JobEventSink: Send + Sync {
    async fn publish(&self, event: JobEvent) -> JobResult<()>;
}
