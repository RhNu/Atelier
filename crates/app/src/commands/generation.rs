use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{
    GenerationStatusDto, GenerationStatusQueryDto, QueueDirectiveDto, RunGenerationJobRequestDto,
    SubmitGenerationRequestDto,
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
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
        Self::command_result(self.current_app()?.generation().submit(request).await)
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
            self.current_app()?
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
        Self::command_result(self.current_app()?.generation().pause().await)
    }

    /// Resumes a paused generation queue.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or no paused queue can be resumed.
    pub async fn resume_generation_queue(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_app()?.generation().resume().await)
    }

    /// Requests a graceful stop for the active generation queue.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or no queue can be stopped.
    pub async fn stop_generation_queue(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_app()?.generation().stop().await)
    }

    /// Notifies the queue that the active delay elapsed.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the queue is not waiting.
    pub async fn generation_delay_elapsed(&self) -> CommandResult<QueueDirectiveDto> {
        Self::command_result(self.current_app()?.generation().delay_elapsed().await)
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
            .current_app()?
            .generation()
            .status(request.job_id.as_deref())
            .await)
    }
}
