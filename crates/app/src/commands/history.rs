use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::history::{
    DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
    DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
    GenerationHistoryBatchDetailDto, GenerationHistoryBatchRequestDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, RerunGenerationHistoryBatchRequestDto,
    RerunGenerationHistoryBatchResponseDto, RerunGenerationHistoryItemRequestDto,
    RerunGenerationHistoryItemResponseDto, RunHistoryPageDto, RunHistoryQueryDto,
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
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
        Self::command_result(self.current_session()?.history().query(request).await)
    }

    /// Queries generation history grouped by the originating batch.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or history cannot be read.
    pub async fn query_generation_history(
        &self,
        request: GenerationHistoryQueryDto,
    ) -> CommandResult<GenerationHistoryPageDto> {
        Self::command_result(
            self.current_session()?
                .history()
                .query_generation(request)
                .await,
        )
    }

    /// Loads ordered request and sample details for one generation batch.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the batch cannot be read.
    pub async fn get_generation_history_batch(
        &self,
        request: GenerationHistoryBatchRequestDto,
    ) -> CommandResult<GenerationHistoryBatchDetailDto> {
        Self::command_result(
            self.current_session()?
                .history()
                .get_generation_batch(request)
                .await,
        )
    }

    /// Deletes run history rows and their history output index rows.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or history cannot be updated.
    pub async fn delete_run_history_items(
        &self,
        request: DeleteRunHistoryItemsRequestDto,
    ) -> CommandResult<DeleteRunHistoryItemsResponseDto> {
        Self::command_result(
            self.current_session()?
                .history()
                .delete_items(request)
                .await,
        )
    }

    /// Deletes generation history rows for complete batches without deleting gallery resources.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or history cannot be updated.
    pub async fn delete_generation_history_batches(
        &self,
        request: DeleteGenerationHistoryBatchesRequestDto,
    ) -> CommandResult<DeleteGenerationHistoryBatchesResponseDto> {
        Self::command_result(
            self.current_session()?
                .history()
                .delete_generation_batches(request)
                .await,
        )
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
            self.current_session()?
                .history()
                .rerun_generation(request)
                .await,
        )
    }

    /// Recreates every ordered request from a previous generation batch.
    ///
    /// # Errors
    /// Returns an error envelope when payload validation or atomic queue submission fails.
    pub async fn rerun_generation_history_batch(
        &self,
        request: RerunGenerationHistoryBatchRequestDto,
    ) -> CommandResult<RerunGenerationHistoryBatchResponseDto> {
        Self::command_result(
            self.current_session()?
                .history()
                .rerun_generation_batch(request)
                .await,
        )
    }
}
