use atelier_jobs::{
    BatchStatus, JobId, JobKind, JobPayloadRef, JobQueue, JobQueueSnapshot, JobStatus,
    QueueDirective, RetryPolicy, SubmitJob,
};
use atelier_precise_reference::PreciseReferenceInput;

use crate::{
    EnsureVibeEncoding, EnsuredVibeEncoding, ExportVibeDocument, ExportedVibeDocument,
    GenerationPayloadStore, ImportEmbeddedPngVibeDocument, ImportVibeDocument,
    ImportedVibeDocuments, KernelClock, KernelDirectorPorts, KernelError, KernelEvent,
    KernelEventKind, KernelEventSink, KernelGenerationPorts, KernelPreciseReferencePorts,
    KernelResult, KernelVibePorts, RanDirectorTool, RunDirectorTool, SubmitGenerationBatch,
    SubmitGenerationBatchJob, SubmitGenerationWork, SubmittedGenerationPayload,
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
            queue,
            ports,
            next_event_sequence: 0,
        })
    }

    #[must_use]
    pub fn queue_snapshot(&self) -> JobQueueSnapshot {
        self.queue.snapshot()
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
    P: KernelEventSink,
{
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
        let mut candidate_queue = self.queue.clone();
        let directive = candidate_queue.submit_batch(batch.batch_id.clone(), jobs)?;
        self.ports.save_submitted_payloads(payloads).await?;
        self.queue = candidate_queue;
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
        impact: atelier_jobs::JobFailureImpact,
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
}

impl<P> KernelRuntime<P>
where
    P: KernelClock + KernelDirectorPorts + KernelEventSink,
{
    /// Runs one Director tool request and indexes the produced image.
    ///
    /// # Errors
    /// Returns an error when the Director client fails or persistence/indexing
    /// cannot complete.
    pub async fn run_director_tool(
        &mut self,
        request: RunDirectorTool,
    ) -> KernelResult<RanDirectorTool> {
        crate::workflow::director::run_director_tool(self, request).await
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
    ) -> KernelResult<atelier_generation::CharacterReference> {
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
