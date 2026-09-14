use super::generation::GenerationUseCases;
use crate::usecases::history::generation_history_records_from_queue_snapshot;
use crate::{AppError, AppResult};
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_jobs::GenerationStore;
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub(crate) async fn commit_submission(
        &self,
        snapshot: &atelier_jobs::JobQueueSnapshot,
        records: Vec<atelier_jobs::RunHistoryRecord>,
        previous: atelier_jobs::JobQueueSnapshot,
        kernel: &mut atelier_kernel::KernelRuntime<crate::ports::AppKernelPorts<S, F, E>>,
    ) -> AppResult<()> {
        if let Err(error) = self
            .app
            .queue_repository
            .commit(Some(snapshot), records)
            .await
        {
            kernel.restore_queue_snapshot(previous)?;
            return Err(AppError::new("job_queue", error.to_string()));
        }
        self.app.events.notify_changed();
        Ok(())
    }

    pub(crate) async fn persist_queue_snapshot(
        &self,
        directive: &QueueDirectiveDto,
        snapshot: &atelier_jobs::JobQueueSnapshot,
    ) -> AppResult<()> {
        let history =
            generation_history_records_from_queue_snapshot(&self.app.run_history, snapshot).await?;
        let durable_snapshot = (!matches!(directive, QueueDirectiveDto::Idle)).then_some(snapshot);
        self.app
            .queue_repository
            .commit(durable_snapshot, history)
            .await
            .map_err(|error| AppError::new("job_queue", error.to_string()))?;
        self.app.events.notify_changed();
        Ok(())
    }

    pub(crate) async fn persist_or_restore(
        &self,
        directive: &QueueDirectiveDto,
        snapshot: &atelier_jobs::JobQueueSnapshot,
        previous_snapshot: atelier_jobs::JobQueueSnapshot,
        kernel: &mut atelier_kernel::KernelRuntime<crate::ports::AppKernelPorts<S, F, E>>,
    ) -> AppResult<()> {
        if let Err(error) = self.persist_queue_snapshot(directive, snapshot).await {
            kernel.restore_queue_snapshot(previous_snapshot)?;
            return Err(error);
        }
        Ok(())
    }

    pub(crate) async fn persist_queue_snapshot_after_failure(
        &self,
        snapshot: &atelier_jobs::JobQueueSnapshot,
    ) -> AppResult<()> {
        let history =
            generation_history_records_from_queue_snapshot(&self.app.run_history, snapshot).await?;
        self.app
            .queue_repository
            .commit(Some(snapshot), history)
            .await
            .map_err(|error| AppError::new("job_queue", error.to_string()))?;
        self.app.events.notify_changed();
        Ok(())
    }
}
