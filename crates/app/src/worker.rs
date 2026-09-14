use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use atelier_app_api::generation::{QueueDelayDto, QueueDirectiveDto};
use futures_timer::Delay;

use crate::commands::{AtelierRuntime, CommandResult};
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

#[derive(Clone, Debug, Default)]
pub struct GenerationWorkerCancel {
    cancelled: Arc<AtomicBool>,
    active_cancelled: Arc<AtomicBool>,
}

impl GenerationWorkerCancel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn cancel_active(&self) {
        self.cancel();
        self.active_cancelled.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl atelier_kernel::GenerationTaskCancellation for GenerationWorkerCancel {
    fn is_cancelled(&self) -> bool {
        self.active_cancelled.load(Ordering::SeqCst)
    }
}

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    /// Drives generation queue directives until the queue stops or cancellation is requested.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or queue execution fails.
    pub async fn drive_generation_queue(
        &self,
        mut directive: QueueDirectiveDto,
        cancel: GenerationWorkerCancel,
    ) -> CommandResult<QueueDirectiveDto> {
        let session = self.current_session()?;
        let Some(active) = session.queue.snapshot().active_batch else {
            return Ok(QueueDirectiveDto::Idle);
        };
        if active.batch.status.is_terminal()
            || matches!(&directive, QueueDirectiveDto::StartJob { job_id } if !active.batch.jobs.iter().any(|job| job.job_id.as_str() == job_id))
        {
            return Ok(QueueDirectiveDto::Idle);
        }
        let batch_id = active.batch.batch_id;
        loop {
            if !session.queue.snapshot().active_batch.is_some_and(|active| {
                active.batch.batch_id == batch_id && !active.batch.status.is_terminal()
            }) {
                return Ok(QueueDirectiveDto::Idle);
            }
            if !self
                .current_session_optional()?
                .is_some_and(|current| Arc::ptr_eq(&current, &session))
            {
                return Ok(QueueDirectiveDto::Idle);
            }
            if cancel.is_cancelled() {
                return Ok(directive);
            }
            match directive {
                QueueDirectiveDto::StartJob { job_id } => {
                    directive = Self::command_result(
                        session
                            .generation()
                            .run_job_cancellable(&job_id, &cancel)
                            .await,
                    )?;
                }
                QueueDirectiveDto::Wait { delay } => {
                    if !wait_for_queue_delay(delay, &cancel).await {
                        return Ok(QueueDirectiveDto::Wait { delay });
                    }
                    if !self
                        .current_session_optional()?
                        .is_some_and(|current| Arc::ptr_eq(&current, &session))
                    {
                        return Ok(QueueDirectiveDto::Idle);
                    }
                    directive = Self::command_result(
                        session
                            .generation()
                            .delay_elapsed_for_batch(batch_id.as_str())
                            .await,
                    )?;
                }
                QueueDirectiveDto::Paused | QueueDirectiveDto::Idle => return Ok(directive),
            }
        }
    }
}

async fn wait_for_queue_delay(delay: QueueDelayDto, cancel: &GenerationWorkerCancel) -> bool {
    let total = Duration::from_millis(delay.max_ms);
    if total.is_zero() {
        return !cancel.is_cancelled();
    }
    let deadline = Instant::now() + total;
    loop {
        if cancel.is_cancelled() {
            return false;
        }
        let now = Instant::now();
        if now >= deadline {
            return true;
        }
        let remaining = deadline.saturating_duration_since(now);
        Delay::new(remaining.min(Duration::from_millis(10))).await;
    }
}
