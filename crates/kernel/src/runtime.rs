// Mutable execution borrows prevent overlapping workflows; QueueView exposes reads only.
#![allow(clippy::needless_pass_by_ref_mut)]

use atelier_jobs::{
    BatchStatus, JobId, JobKind, JobPayloadRef, JobQueue, JobQueueSnapshot, JobStatus,
    QueueDirective, RetryPolicy, SubmitJob,
};

use crate::{
    GenerationPayloadStore, KernelClock, KernelError, KernelEventKind, KernelEventSink,
    KernelGenerationPorts, KernelResult, SubmitGenerationBatch, SubmitGenerationBatchJob,
    SubmitGenerationWork, SubmittedGenerationPayload,
};

pub trait GenerationTaskCancellation: Send + Sync {
    fn is_cancelled(&self) -> bool;
}

struct NeverCancel;

impl GenerationTaskCancellation for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct KernelRuntime<P> {
    queue: crate::QueueView,
    context: crate::WorkflowContext<P>,
}

impl<P> KernelRuntime<P> {
    #[must_use]
    pub fn new(ports: P) -> Self {
        Self::with_retry_policy(ports, RetryPolicy::default())
    }

    #[must_use]
    pub fn with_retry_policy(ports: P, retry_policy: RetryPolicy) -> Self {
        Self {
            queue: crate::QueueView::new(JobQueue::new(retry_policy)),
            context: crate::WorkflowContext::new(ports),
        }
    }

    /// Restores a runtime from a persisted queue snapshot and converts any
    /// in-flight work into a user-resumable paused state.
    ///
    /// # Errors
    /// Returns an error when the queue snapshot is internally inconsistent.
    pub fn from_recovered_queue_snapshot(
        ports: P,
        snapshot: JobQueueSnapshot,
    ) -> KernelResult<Self> {
        let mut queue = JobQueue::from_snapshot(snapshot)?;
        queue.recover_after_restart()?;
        Ok(Self {
            queue: crate::QueueView::new(queue),
            context: crate::WorkflowContext::new(ports),
        })
    }

    #[must_use]
    pub fn queue_snapshot(&self) -> JobQueueSnapshot {
        self.queue.lock().snapshot()
    }

    /// Replaces the in-memory queue with a previously captured snapshot.
    ///
    /// This is used by the application facade when durable persistence fails
    /// after a state transition, keeping the live runtime aligned with the
    /// last committed state.
    ///
    /// # Errors
    /// Returns an error when the supplied snapshot is internally inconsistent.
    pub fn restore_queue_snapshot(&mut self, snapshot: JobQueueSnapshot) -> KernelResult<()> {
        *self.queue.lock() = JobQueue::from_snapshot(snapshot)?;
        Ok(())
    }

    #[must_use]
    pub fn ports(&self) -> &P {
        self.context.ports()
    }

    #[must_use]
    pub const fn context(&self) -> &crate::WorkflowContext<P> {
        &self.context
    }

    #[must_use]
    pub fn queue_view(&self) -> crate::QueueView {
        self.queue.clone()
    }

    #[must_use]
    pub fn batch_status(&self) -> Option<BatchStatus> {
        self.queue.lock().batch_status()
    }

    #[must_use]
    pub fn job_status(&self, job_id: &JobId) -> Option<JobStatus> {
        self.queue.lock().job_status(job_id)
    }

    /// Requests a pause without cancelling a running job.
    ///
    /// # Errors
    /// Returns an error when the active queue cannot be paused.
    pub fn pause(&mut self) -> KernelResult<QueueDirective> {
        self.queue.lock().pause().map_err(KernelError::from)
    }

    /// Resumes a paused batch.
    ///
    /// # Errors
    /// Returns an error when no paused batch can be resumed.
    pub fn resume(&mut self) -> KernelResult<QueueDirective> {
        self.queue.lock().resume().map_err(KernelError::from)
    }

    /// Requests a graceful stop for the active batch.
    ///
    /// # Errors
    /// Returns an error when there is no stoppable active batch.
    pub fn stop(&mut self) -> KernelResult<QueueDirective> {
        self.queue.lock().stop().map_err(KernelError::from)
    }

    /// Tells the queue that the current delay elapsed.
    ///
    /// # Errors
    /// Returns an error when the active queue is not waiting on a delay.
    pub fn delay_elapsed(&mut self) -> KernelResult<QueueDirective> {
        self.queue.lock().delay_elapsed().map_err(KernelError::from)
    }
}

impl<P: KernelEventSink> KernelRuntime<P> {
    pub(crate) async fn emit(&self, kind: KernelEventKind) {
        self.context.emit(kind).await;
    }
}

impl<P> KernelRuntime<P>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    /// Stores a submitted generation payload and enqueues its first job.
    ///
    /// # Errors
    /// Returns an error when payload storage fails or the single queue rejects
    /// the batch.
    pub async fn submit_generation_work(
        &mut self,
        work: SubmitGenerationWork,
    ) -> KernelResult<QueueDirective> {
        self.submit_generation_batch(SubmitGenerationBatch {
            batch_id: work.batch_id,
            jobs: vec![SubmitGenerationBatchJob {
                compiled_prompt: work.compiled_prompt,
                job_id: work.job_id,
                request: work.request,
            }],
            context: work.context,
        })
        .await
    }

    /// Stores submitted generation payloads and enqueues a multi-job batch.
    ///
    /// # Errors
    /// Returns an error when payload storage fails or the queue rejects the batch.
    pub async fn submit_generation_batch(
        &mut self,
        batch: SubmitGenerationBatch,
    ) -> KernelResult<QueueDirective> {
        let mut payloads = Vec::with_capacity(batch.jobs.len());
        let mut jobs = Vec::with_capacity(batch.jobs.len());
        for job in batch.jobs {
            let payload_ref = submitted_payload_ref(&job.job_id);
            payloads.push(SubmittedGenerationPayload {
                compiled_prompt: job.compiled_prompt,
                payload_ref: payload_ref.clone(),
                batch_id: batch.batch_id.clone(),
                job_id: job.job_id.clone(),
                request: job.request,
                context: batch.context,
            });
            jobs.push(SubmitJob {
                job_id: job.job_id,
                kind: JobKind::GenerateImage,
                payload_ref,
            });
        }
        let mut candidate_queue = self.queue.lock().clone();
        let directive = candidate_queue.submit_batch(batch.batch_id.clone(), jobs)?;
        self.context
            .ports()
            .save_submitted_payloads(payloads)
            .await?;
        *self.queue.lock() = candidate_queue;
        self.emit(KernelEventKind::BatchSubmitted {
            batch_id: batch.batch_id,
        })
        .await;
        Ok(directive)
    }

    /// Runs the currently scheduled generation job.
    ///
    /// # Errors
    /// Returns an error when preparation, planning, persistence, or indexing
    /// fails. `NovelAI` retryable failures are converted into queue directives.
    pub async fn run_scheduled_generation_job(
        &mut self,
        job_id: &JobId,
    ) -> KernelResult<QueueDirective> {
        self.run_scheduled_generation_job_cancellable(job_id, &NeverCancel)
            .await
    }

    /// Runs one scheduled generation job with an out-of-band active-stream cancellation signal.
    ///
    /// # Errors
    /// Returns an error when generation fails or the cancellation signal stops an active stream.
    pub async fn run_scheduled_generation_job_cancellable(
        &mut self,
        job_id: &JobId,
        cancellation: &dyn GenerationTaskCancellation,
    ) -> KernelResult<QueueDirective> {
        crate::workflow::generation::run_scheduled_generation_job(self, job_id, cancellation).await
    }

    pub(crate) fn mark_preparing(&mut self, job_id: &JobId) -> KernelResult<QueueDirective> {
        self.queue
            .lock()
            .mark_preparing(job_id)
            .map_err(KernelError::from)
    }

    pub(crate) fn mark_running(
        &mut self,
        job_id: &JobId,
        payload_ref: JobPayloadRef,
    ) -> KernelResult<QueueDirective> {
        self.queue
            .lock()
            .mark_running(job_id, payload_ref)
            .map_err(KernelError::from)
    }

    pub(crate) fn mark_succeeded(&mut self, job_id: &JobId) -> KernelResult<QueueDirective> {
        self.queue
            .lock()
            .mark_succeeded(job_id)
            .map_err(KernelError::from)
    }

    pub(crate) fn mark_failed(
        &mut self,
        job_id: &JobId,
        impact: atelier_jobs::JobFailureImpact,
    ) -> KernelResult<QueueDirective> {
        self.queue
            .lock()
            .mark_failed(job_id, impact)
            .map_err(KernelError::from)
    }

    pub(crate) fn retry_policy(&self) -> RetryPolicy {
        self.queue.lock().retry_policy()
    }
}

#[must_use]
pub fn submitted_payload_ref(job_id: &JobId) -> JobPayloadRef {
    JobPayloadRef::new(format!("generation-submitted:{}", job_id.as_str()))
}

#[must_use]
pub fn prepared_payload_ref(job_id: &JobId) -> JobPayloadRef {
    JobPayloadRef::new(format!("generation-prepared:{}", job_id.as_str()))
}
