use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use super::generation::GenerationUseCases;
use super::{AppError, AppResult, generation_history_records_from_queue_snapshot};

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub(crate) async fn persist_queue_snapshot(
        &self,
        directive: &QueueDirectiveDto,
        snapshot: &atelier_jobs::JobQueueSnapshot,
    ) -> AppResult<()> {
        let history =
            generation_history_records_from_queue_snapshot(&self.app.inner.run_history, snapshot)
                .await?;
        let durable_snapshot = (!matches!(directive, QueueDirectiveDto::Idle)).then_some(snapshot);
        self.app
            .inner
            .queue_repository
            .commit_queue_and_history(durable_snapshot, history)
            .map_err(|error| AppError::new("job_queue", error.to_string()))
    }

    pub(crate) async fn persist_or_restore(
        &self,
        directive: &QueueDirectiveDto,
        snapshot: &atelier_jobs::JobQueueSnapshot,
        previous_snapshot: atelier_jobs::JobQueueSnapshot,
    ) -> AppResult<()> {
        if let Err(error) = self.persist_queue_snapshot(directive, snapshot).await {
            let _ = self
                .app
                .inner
                .kernel
                .lock()
                .await
                .restore_queue_snapshot(previous_snapshot);
            return Err(error);
        }
        Ok(())
    }

    pub(crate) async fn persist_queue_snapshot_after_failure(
        &self,
        snapshot: &atelier_jobs::JobQueueSnapshot,
    ) -> AppResult<()> {
        let history =
            generation_history_records_from_queue_snapshot(&self.app.inner.run_history, snapshot)
                .await?;
        self.app
            .inner
            .queue_repository
            .commit_queue_and_history(Some(snapshot), history)
            .map_err(|error| AppError::new("job_queue", error.to_string()))
    }
}
