use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::history::{
    RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto, RunHistoryPageDto,
    RunHistoryQueryDto,
};
use nai_atelier_secrets::SecretStore;
use nai_atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    /// Queries durable generation and Director run history.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or history cannot be read.
    pub async fn query_run_history(
        &self,
        request: RunHistoryQueryDto,
    ) -> CommandResult<RunHistoryPageDto> {
        Self::command_result(self.current_app()?.history().query(request).await)
    }

    /// Creates a new generation job from a previous generation history item.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the history item is
    /// not rerunnable, no active API key is available, or the queue rejects the job.
    pub async fn rerun_generation_history_item(
        &self,
        request: RerunGenerationHistoryItemRequestDto,
    ) -> CommandResult<RerunGenerationHistoryItemResponseDto> {
        Self::command_result(
            self.current_app()?
                .history()
                .rerun_generation(request)
                .await,
        )
    }
}
