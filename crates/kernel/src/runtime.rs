use nai_atelier_jobs::{
    BatchStatus, JobId, JobKind, JobPayloadRef, JobQueue, JobStatus, QueueDirective, RetryPolicy,
    SubmitJob,
};
use nai_atelier_precise_reference::PreciseReferenceInput;

use crate::{
    EnsureVibeEncoding, EnsuredVibeEncoding, ExportVibeDocument, ExportedVibeDocument,
    GenerationPayloadStore, ImportEmbeddedPngVibeDocument, ImportVibeDocument,
    ImportedVibeDocuments, KernelClock, KernelError, KernelEvent, KernelEventKind, KernelEventSink,
    KernelGenerationPorts, KernelPreciseReferencePorts, KernelResult, KernelVibePorts,
    SubmitGenerationWork, SubmittedGenerationPayload,
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
        Self::with_retry_policy(ports, RetryPolicy::default())
    }

    #[must_use]
    pub const fn with_retry_policy(ports: P, retry_policy: RetryPolicy) -> Self {
        Self {
            queue: JobQueue::new(retry_policy),
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

    pub(crate) const fn retry_policy(&self) -> RetryPolicy {
        self.queue.retry_policy()
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

impl<P> KernelRuntime<P>
where
    P: KernelVibePorts,
{
    /// Imports official Vibe JSON and registers its document resources.
    ///
    /// # Errors
    /// Returns an error when the document is invalid, resource registration
    /// fails, or repository persistence fails.
    pub async fn import_vibe_document(
        &self,
        request: ImportVibeDocument,
    ) -> KernelResult<ImportedVibeDocuments> {
        crate::workflow::vibe::import_vibe_document(self, request).await
    }

    /// Extracts an embedded Vibe document from PNG bytes and imports it.
    ///
    /// # Errors
    /// Returns an error when extraction, import, resource registration, or
    /// repository persistence fails.
    pub async fn import_embedded_png_vibe_document(
        &self,
        request: ImportEmbeddedPngVibeDocument,
    ) -> KernelResult<ImportedVibeDocuments> {
        crate::workflow::vibe::import_embedded_png_vibe_document(self, request).await
    }

    /// Exports one or more managed Vibes as official JSON.
    ///
    /// # Errors
    /// Returns an error when a Vibe cannot be found, its document resource
    /// cannot be read, or the requested format is invalid for the selection.
    pub async fn export_vibe_document(
        &self,
        request: ExportVibeDocument,
    ) -> KernelResult<ExportedVibeDocument> {
        crate::workflow::vibe::export_vibe_document(self, request).await
    }

    /// Ensures a model/settings-specific Vibe encoding exists for a source image.
    ///
    /// # Errors
    /// Returns an error when cache lookup, `NovelAI` encoding, resource
    /// registration, or cache persistence fails.
    pub async fn ensure_vibe_encoding(
        &self,
        request: EnsureVibeEncoding,
    ) -> KernelResult<EnsuredVibeEncoding> {
        crate::workflow::vibe::ensure_vibe_encoding(self, request).await
    }
}

impl<P> KernelRuntime<P>
where
    P: KernelPreciseReferencePorts,
{
    /// Resolves a resource-backed precise reference into a generation input.
    ///
    /// # Errors
    /// Returns an error when the source resource cannot be resolved or is not a
    /// valid precise-reference image.
    pub async fn prepare_precise_reference(
        &self,
        input: PreciseReferenceInput,
    ) -> KernelResult<nai_atelier_generation::CharacterReference> {
        crate::workflow::precise_reference::prepare_precise_reference(self, input).await
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
