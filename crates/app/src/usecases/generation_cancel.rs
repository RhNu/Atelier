use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use super::GenerationUseCases;
use crate::AppResult;

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub(crate) async fn delay_elapsed_for_batch(
        &self,
        batch_id: &str,
    ) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.kernel.lock().await;
        let previous = kernel.queue_snapshot();
        if !previous.active_batch.as_ref().is_some_and(|active| {
            active.batch.batch_id.as_str() == batch_id
                && active.batch.status == atelier_jobs::BatchStatus::Waiting
        }) {
            return Ok(QueueDirectiveDto::Idle);
        }
        let directive = crate::mapping::queue_directive_to_dto(kernel.delay_elapsed()?);
        let snapshot = kernel.queue_snapshot();
        self.persist_or_restore(&directive, &snapshot, previous, &mut kernel)
            .await?;
        drop(kernel);
        Ok(directive)
    }

    /// Called after signaling the owned request. The execution lock drains its
    /// network future and durable output writes before changing queue state.
    pub(crate) async fn cancel_batch(&self, batch_id: &str) -> AppResult<()> {
        let mut kernel = self.app.kernel.lock().await;
        let previous = kernel.queue_snapshot();
        if kernel.cancel_batch(&atelier_jobs::BatchId::new(batch_id))? {
            let snapshot = kernel.queue_snapshot();
            self.persist_or_restore(&QueueDirectiveDto::Idle, &snapshot, previous, &mut kernel)
                .await?;
        }
        drop(kernel);
        self.app.agent_turn.generations.remove(batch_id)?;
        Ok(())
    }
}

pub(super) struct GenerationCancellation<'a> {
    pub worker: &'a dyn atelier_kernel::GenerationTaskCancellation,
    pub agent: Option<tokio_util::sync::CancellationToken>,
}

impl atelier_kernel::GenerationTaskCancellation for GenerationCancellation<'_> {
    fn is_cancelled(&self) -> bool {
        self.worker.is_cancelled()
            || self
                .agent
                .as_ref()
                .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    }
}
