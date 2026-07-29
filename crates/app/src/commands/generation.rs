use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{
    GenerationAnlasEstimateDto, GenerationDraftDto, GenerationEstimateRequestDto,
    GenerationStatusDto, GenerationStatusQueryDto, QueueDirectiveDto, RunGenerationJobRequestDto,
    SaveGenerationDraftRequestDto, SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto,
};
use atelier_app_api::prompt::AppendLexiconEntitiesRequestDto;
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    /// Returns the persisted generation workbench draft for the current workspace.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the draft cannot be decoded.
    pub async fn get_generation_draft(&self) -> CommandResult<Option<GenerationDraftDto>> {
        Self::command_result(self.current_session()?.generation().get_draft().await)
    }

    /// Validates and persists the generation workbench draft for the current workspace.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, validation fails, or persistence fails.
    pub async fn save_generation_draft(
        &self,
        request: SaveGenerationDraftRequestDto,
    ) -> CommandResult<GenerationDraftDto> {
        Self::command_result(
            self.current_session()?
                .generation()
                .save_draft(request)
                .await,
        )
    }

    /// Clears the persisted generation workbench draft for the current workspace.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or draft resource cleanup fails.
    pub async fn clear_generation_draft(&self) -> CommandResult<()> {
        Self::command_result(self.current_session()?.generation().clear_draft().await)
    }

    /// Resolves lexicon entities and atomically appends canonical tags to the persisted draft.
    ///
    /// # Errors
    /// Returns an error envelope when an entity is invalid, no workspace is open, or persistence fails.
    pub async fn append_lexicon_entities_to_generation_draft(
        &self,
        request: AppendLexiconEntitiesRequestDto,
    ) -> CommandResult<GenerationDraftDto> {
        let entities = self
            .lexicon
            .resolve_entities(&request.entity_ids)
            .map_err(crate::AppError::from)
            .map_err(|error| error.envelope())?;
        Self::command_result(
            self.current_session()?
                .generation()
                .append_lexicon_entities(request.target, &entities)
                .await,
        )
    }

    /// Submits generation work without running the queued job.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, no API key is active, or queue persistence fails.
    pub async fn submit_generation(
        &self,
        request: SubmitGenerationRequestDto,
    ) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_session()?.generation().submit(request).await)
    }

    /// Submits a multi-job generation batch without running queued jobs inline.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, no API key is active, or queue persistence fails.
    pub async fn submit_generation_batch(
        &self,
        request: SubmitGenerationBatchRequestDto,
    ) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(
            self.current_session()?
                .generation()
                .submit_batch(request)
                .await,
        )
    }

    /// Estimates `NovelAI` Anlas cost for a generation request.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the estimate request is invalid.
    pub async fn estimate_generation(
        &self,
        request: GenerationEstimateRequestDto,
    ) -> CommandResult<GenerationAnlasEstimateDto> {
        Self::command_result(self.current_session()?.generation().estimate(request).await)
    }

    /// Runs one scheduled generation job.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or generation execution fails.
    pub async fn run_generation_job(
        &self,
        request: RunGenerationJobRequestDto,
    ) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(
            self.current_session()?
                .generation()
                .run_job(&request.job_id)
                .await,
        )
    }

    /// Pauses the active generation queue.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the queue cannot be paused.
    pub async fn pause_generation_queue(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_session()?.generation().pause().await)
    }

    /// Resumes a paused generation queue.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or no paused queue can be resumed.
    pub async fn resume_generation_queue(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_session()?.generation().resume().await)
    }

    /// Requests a graceful stop for the active generation queue.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or no queue can be stopped.
    pub async fn stop_generation_queue(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_session()?.generation().stop().await)
    }

    /// Notifies the queue that the active delay elapsed.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the queue is not waiting.
    pub async fn generation_delay_elapsed(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_session()?.generation().delay_elapsed().await)
    }

    /// Returns current generation queue and optional job status.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open.
    pub async fn generation_status(
        &self,
        request: GenerationStatusQueryDto,
    ) -> CommandResult<GenerationStatusDto> {
        Self::command_result(
            self.current_session()?
                .generation()
                .status(request.job_id.as_deref())
                .await,
        )
    }
}
