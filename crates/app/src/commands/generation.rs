use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{
    GenerationAnlasEstimateDto, GenerationEstimateRequestDto, GenerationStatusDto,
    GenerationStatusQueryDto, QueueDirectiveDto, RunGenerationJobRequestDto,
    SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto,
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
        Ok(self
            .current_session()?
            .generation()
            .status(request.job_id.as_deref())
            .await)
    }
}
