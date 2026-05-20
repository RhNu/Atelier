use nai_atelier_jobs::{
    BatchStatus, JobId, JobKind, JobPayloadRef, JobQueue, JobStatus, QueueDirective, SubmitJob,
};

use crate::{
    GenerationPayloadStore, KernelClock, KernelError, KernelEvent, KernelEventKind,
    KernelEventSink, KernelGenerationPorts, KernelResult, SubmitGenerationWork,
    SubmittedGenerationPayload,
};

#[derive(Clone, Debug)]
pub struct KernelRuntime<P> {
    queue: JobQueue,
    ports: P,
    next_event_sequence: u64,
}

impl<P> KernelRuntime<P> {
    #[must_use]
    pub fn new(ports: P) -> Self {
        Self {
            queue: JobQueue::default(),
            ports,
            next_event_sequence: 0,
        }
    }

    #[must_use]
    pub const fn ports(&self) -> &P {
        &self.ports
    }

    #[must_use]
    pub fn batch_status(&self) -> Option<BatchStatus> {
        self.queue.batch_status()
    }

    #[must_use]
    pub fn job_status(&self, job_id: &JobId) -> Option<JobStatus> {
        self.queue.job_status(job_id)
    }

    /// Requests a pause without cancelling a running job.
    ///
    /// # Errors
    /// Returns an error when the active queue cannot be paused.
    pub fn pause(&mut self) -> KernelResult<QueueDirective> {
        self.queue.pause().map_err(KernelError::from)
    }

    /// Resumes a paused batch.
    ///
    /// # Errors
    /// Returns an error when no paused batch can be resumed.
    pub fn resume(&mut self) -> KernelResult<QueueDirective> {
        self.queue.resume().map_err(KernelError::from)
    }

    /// Requests a graceful stop for the active batch.
    ///
    /// # Errors
    /// Returns an error when there is no stoppable active batch.
    pub fn stop(&mut self) -> KernelResult<QueueDirective> {
        self.queue.stop().map_err(KernelError::from)
    }

    /// Tells the queue that the current delay elapsed.
    ///
    /// # Errors
    /// Returns an error when the active queue is not waiting on a delay.
    pub fn delay_elapsed(&mut self) -> KernelResult<QueueDirective> {
        self.queue.delay_elapsed().map_err(KernelError::from)
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
        let payload_ref = submitted_payload_ref(&work.job_id);
        let payload = SubmittedGenerationPayload {
            payload_ref: payload_ref.clone(),
            batch_id: work.batch_id.clone(),
            job_id: work.job_id.clone(),
            request: work.request,
            context: work.context,
        };
        let mut candidate_queue = self.queue.clone();
        let directive = candidate_queue.submit_batch(
            work.batch_id.clone(),
            vec![SubmitJob {
                job_id: work.job_id,
                kind: JobKind::GenerateImage,
                payload_ref,
            }],
        )?;
        self.ports.save_submitted_payload(payload).await?;
        self.queue = candidate_queue;
        self.emit(KernelEventKind::BatchSubmitted {
            batch_id: work.batch_id,
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
        crate::workflow::generation::run_scheduled_generation_job(self, job_id).await
    }

    pub(crate) fn mark_preparing(&mut self, job_id: &JobId) -> KernelResult<QueueDirective> {
        self.queue.mark_preparing(job_id).map_err(KernelError::from)
    }

    pub(crate) fn mark_running(
        &mut self,
        job_id: &JobId,
        payload_ref: JobPayloadRef,
    ) -> KernelResult<QueueDirective> {
        self.queue
            .mark_running(job_id, payload_ref)
            .map_err(KernelError::from)
    }

    pub(crate) fn mark_succeeded(&mut self, job_id: &JobId) -> KernelResult<QueueDirective> {
        self.queue.mark_succeeded(job_id).map_err(KernelError::from)
    }

    pub(crate) fn mark_failed(
        &mut self,
        job_id: &JobId,
        impact: nai_atelier_jobs::JobFailureImpact,
    ) -> KernelResult<QueueDirective> {
        self.queue
            .mark_failed(job_id, impact)
            .map_err(KernelError::from)
    }

    pub(crate) const fn ports_ref(&self) -> &P {
        &self.ports
    }

    pub(crate) async fn emit(&mut self, kind: KernelEventKind) {
        self.next_event_sequence += 1;
        self.ports
            .emit(KernelEvent {
                sequence: self.next_event_sequence,
                kind,
            })
            .await;
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
