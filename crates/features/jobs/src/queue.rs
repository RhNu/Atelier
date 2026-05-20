use crate::{
    BatchId, BatchStatus, JobBatch, JobFailureImpact, JobId, JobPayloadRef, JobQueueError,
    JobRecord, JobResult, JobStatus, QueueDelay, QueueDirective, RetryPolicy, SubmitJob,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JobQueue {
    active_batch: Option<ActiveBatch>,
    retry_policy: RetryPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveBatch {
    batch: JobBatch,
    current_job: Option<JobId>,
    pending_delay: Option<QueueDelay>,
    paused_delay: Option<QueueDelay>,
    pause_after_current: bool,
    stop_after_current: bool,
}

impl JobQueue {
    #[must_use]
    pub const fn new(retry_policy: RetryPolicy) -> Self {
        Self {
            active_batch: None,
            retry_policy,
        }
    }

    #[must_use]
    pub const fn retry_policy(&self) -> RetryPolicy {
        self.retry_policy
    }

    /// Submits a batch into the single queue and schedules its first job.
    ///
    /// # Errors
    /// Returns an error when another batch is still active or the new batch is empty.
    pub fn submit_batch(
        &mut self,
        batch_id: BatchId,
        jobs: Vec<SubmitJob>,
    ) -> JobResult<QueueDirective> {
        if self.has_active_batch() {
            return Err(JobQueueError::conflict(
                "single queue already has an active batch",
            ));
        }
        if jobs.is_empty() {
            return Err(JobQueueError::empty_batch(
                "batch must contain at least one job",
            ));
        }
        if has_duplicate_job_ids(&jobs) {
            return Err(JobQueueError::conflict(
                "batch cannot contain duplicate job ids",
            ));
        }

        let first_job = jobs[0].job_id.clone();
        let batch = JobBatch {
            batch_id,
            status: BatchStatus::Running,
            jobs: jobs.into_iter().map(JobRecord::from).collect(),
        };
        self.active_batch = Some(ActiveBatch {
            batch,
            current_job: None,
            pending_delay: None,
            paused_delay: None,
            pause_after_current: false,
            stop_after_current: false,
        });
        Ok(QueueDirective::StartJob(first_job))
    }

    /// Schedules the next queued job in the active batch.
    ///
    /// # Errors
    /// Returns an error when there is no active batch.
    pub fn start_next(&mut self) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        if batch.batch.status != BatchStatus::Running
            || batch.pending_delay.is_some()
            || batch.paused_delay.is_some()
            || batch.current_job.is_some()
            || retry_waiting_job(&batch.batch.jobs).is_some()
        {
            return Err(JobQueueError::invalid_state(
                "next job cannot start until the queue is ready",
            ));
        }
        let Some(job_id) = next_queued_job(&batch.batch.jobs) else {
            batch.batch.status = BatchStatus::Succeeded;
            return Ok(QueueDirective::Idle);
        };
        batch.batch.status = BatchStatus::Running;
        Ok(QueueDirective::StartJob(job_id))
    }

    /// Marks a job as preparing before its prepared payload is available.
    ///
    /// # Errors
    /// Returns an error when the job is missing or not in a startable state.
    pub fn mark_preparing(&mut self, job_id: &JobId) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        ensure_job_can_start(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        if !matches!(
            job.status,
            JobStatus::Queued | JobStatus::WaitingRetry | JobStatus::Blocked
        ) {
            return Err(JobQueueError::invalid_state(
                "only startable jobs can enter preparing",
            ));
        }
        job.status = JobStatus::Preparing;
        batch.current_job = Some(job_id.clone());
        if batch.batch.status != BatchStatus::Stopping {
            batch.batch.status = BatchStatus::Running;
        }
        Ok(QueueDirective::StartJob(job_id.clone()))
    }

    /// Marks a job as running and records its prepared payload reference once.
    ///
    /// # Errors
    /// Returns an error when the job is missing, not startable, or attempts to
    /// change an already recorded prepared payload reference.
    pub fn mark_running(
        &mut self,
        job_id: &JobId,
        prepared_payload_ref: JobPayloadRef,
    ) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        ensure_job_can_start(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        if !matches!(
            job.status,
            JobStatus::Queued
                | JobStatus::Preparing
                | JobStatus::WaitingRetry
                | JobStatus::Blocked
                | JobStatus::Running
        ) {
            return Err(JobQueueError::invalid_state(
                "job cannot enter running state",
            ));
        }
        if let Some(existing) = &job.prepared_payload_ref {
            if existing != &prepared_payload_ref {
                return Err(JobQueueError::conflict(
                    "prepared payload ref cannot change for a job retry",
                ));
            }
        } else {
            job.prepared_payload_ref = Some(prepared_payload_ref);
        }
        job.status = JobStatus::Running;
        batch.current_job = Some(job_id.clone());
        if batch.batch.status != BatchStatus::Stopping {
            batch.batch.status = BatchStatus::Running;
        }
        Ok(QueueDirective::StartJob(job_id.clone()))
    }

    /// Marks the active job as succeeded and returns the next queue directive.
    ///
    /// # Errors
    /// Returns an error when the job is missing or is not currently running.
    pub fn mark_succeeded(&mut self, job_id: &JobId) -> JobResult<QueueDirective> {
        let should_stop;
        let should_pause;
        {
            let batch = self.active_batch_mut()?;
            let job = find_job_mut(batch, job_id)?;
            if !matches!(job.status, JobStatus::Preparing | JobStatus::Running) {
                return Err(JobQueueError::invalid_state(
                    "only preparing or running jobs can succeed",
                ));
            }
            job.status = JobStatus::Succeeded;
            should_stop = batch.stop_after_current;
            should_pause = batch.pause_after_current;
        }

        if should_stop {
            return self.finish_stopped_batch();
        }
        if should_pause {
            return self.pause_after_completed_job();
        }
        self.schedule_after_completed_job()
    }

    /// Applies a failure impact to the active job.
    ///
    /// # Errors
    /// Returns an error when the job is missing or the queue has no active batch.
    pub fn mark_failed(
        &mut self,
        job_id: &JobId,
        impact: JobFailureImpact,
    ) -> JobResult<QueueDirective> {
        let (should_stop, should_pause) = {
            let batch = self.active_batch_mut()?;
            ensure_current_attempt_job(batch, job_id)?;
            (batch.stop_after_current, batch.pause_after_current)
        };

        if should_stop {
            self.mark_current_failed(job_id)?;
            return self.finish_stopped_batch();
        }

        match impact {
            JobFailureImpact::RetryAfter(delay) if should_pause => {
                self.mark_rate_limited_paused(job_id, delay)
            }
            JobFailureImpact::RetryAfter(delay) => self.mark_rate_limited(job_id, delay),
            JobFailureImpact::FailCurrentAndContinue => {
                self.mark_current_failed(job_id)?;
                if should_pause {
                    return self.pause_after_completed_job();
                }
                self.schedule_after_completed_job()
            }
            JobFailureImpact::PauseAndRetryCurrent => self.block_current_job(job_id),
        }
    }

    /// Tells the queue that its current delay has elapsed.
    ///
    /// # Errors
    /// Returns an error when no active batch is waiting on a delay.
    pub fn delay_elapsed(&mut self) -> JobResult<QueueDirective> {
        {
            let batch = self.active_batch_mut()?;
            if batch.batch.status != BatchStatus::Waiting {
                return Err(JobQueueError::invalid_state("no queue delay is pending"));
            }
            batch.pending_delay = None;

            if let Some(job_id) = retry_waiting_job(&batch.batch.jobs) {
                batch.current_job = Some(job_id.clone());
                batch.batch.status = BatchStatus::Running;
                return Ok(QueueDirective::StartJob(job_id));
            }
            batch.batch.status = BatchStatus::Running;
        }
        self.start_next()
    }

    /// Requests a pause without cancelling a running job.
    ///
    /// # Errors
    /// Returns an error when there is no active batch or the batch is terminal.
    pub fn pause(&mut self) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        match batch.batch.status {
            BatchStatus::Waiting => {
                batch.paused_delay = batch.pending_delay.take();
                batch.batch.status = BatchStatus::Paused;
            }
            BatchStatus::Running if has_running_job(&batch.batch.jobs) => {
                batch.pause_after_current = true;
            }
            BatchStatus::Running => batch.batch.status = BatchStatus::Paused,
            BatchStatus::Paused => {}
            BatchStatus::Stopping | BatchStatus::Stopped | BatchStatus::Succeeded => {
                return Err(JobQueueError::invalid_state("batch cannot be paused"));
            }
        }
        Ok(QueueDirective::Paused)
    }

    /// Resumes a paused batch.
    ///
    /// # Errors
    /// Returns an error when there is no active batch or the batch is not paused.
    pub fn resume(&mut self) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        if batch.batch.status != BatchStatus::Paused {
            return Err(JobQueueError::invalid_state(
                "only paused batches can resume",
            ));
        }
        if let Some(delay) = batch.paused_delay.take() {
            batch.pending_delay = Some(delay);
            batch.batch.status = BatchStatus::Waiting;
            return Ok(QueueDirective::Wait(delay));
        }
        if let Some(job_id) = blocked_job(&batch.batch.jobs) {
            batch.current_job = Some(job_id.clone());
            batch.batch.status = BatchStatus::Running;
            return Ok(QueueDirective::StartJob(job_id));
        }
        batch.batch.status = BatchStatus::Running;
        self.start_next()
    }

    /// Requests graceful stop for the active batch.
    ///
    /// # Errors
    /// Returns an error when there is no active batch or the batch is already terminal.
    pub fn stop(&mut self) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        match batch.batch.status {
            BatchStatus::Running => {
                if has_running_job(&batch.batch.jobs) {
                    batch.stop_after_current = true;
                    batch.batch.status = BatchStatus::Stopping;
                    Ok(QueueDirective::Paused)
                } else {
                    self.finish_stopped_batch()
                }
            }
            BatchStatus::Waiting | BatchStatus::Paused => self.finish_stopped_batch(),
            BatchStatus::Stopping => Ok(QueueDirective::Paused),
            BatchStatus::Succeeded | BatchStatus::Stopped => {
                Err(JobQueueError::invalid_state("batch already ended"))
            }
        }
    }

    #[must_use]
    pub fn batch_status(&self) -> Option<BatchStatus> {
        self.active_batch.as_ref().map(|batch| batch.batch.status)
    }

    #[must_use]
    pub fn job_status(&self, job_id: &JobId) -> Option<JobStatus> {
        self.find_job(job_id).map(|job| job.status)
    }

    #[must_use]
    pub fn retry_attempts(&self, job_id: &JobId) -> Option<u32> {
        self.find_job(job_id).map(|job| job.retry_attempts)
    }

    #[must_use]
    pub fn prepared_payload_ref(&self, job_id: &JobId) -> Option<&JobPayloadRef> {
        self.find_job(job_id)
            .and_then(|job| job.prepared_payload_ref.as_ref())
    }

    #[must_use]
    pub fn paused_delay(&self) -> Option<QueueDelay> {
        self.active_batch
            .as_ref()
            .and_then(|batch| batch.paused_delay)
    }

    fn has_active_batch(&self) -> bool {
        self.active_batch
            .as_ref()
            .is_some_and(|batch| !batch.batch.status.is_terminal())
    }

    fn active_batch_mut(&mut self) -> JobResult<&mut ActiveBatch> {
        self.active_batch
            .as_mut()
            .ok_or_else(|| JobQueueError::invalid_state("queue has no active batch"))
    }

    fn find_job(&self, job_id: &JobId) -> Option<&JobRecord> {
        self.active_batch
            .as_ref()
            .and_then(|batch| batch.batch.jobs.iter().find(|job| &job.job_id == job_id))
    }

    fn mark_rate_limited(
        &mut self,
        job_id: &JobId,
        delay: QueueDelay,
    ) -> JobResult<QueueDirective> {
        let max_retries = self.retry_policy.max_rate_limit_retries;
        let batch = self.active_batch_mut()?;
        ensure_current_attempt_job(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        if job.retry_attempts >= max_retries {
            job.status = JobStatus::Blocked;
            batch.current_job = Some(job_id.clone());
            batch.batch.status = BatchStatus::Paused;
            return Ok(QueueDirective::Paused);
        }
        job.retry_attempts += 1;
        job.status = JobStatus::WaitingRetry;
        batch.current_job = Some(job_id.clone());
        batch.pending_delay = Some(delay);
        batch.batch.status = BatchStatus::Waiting;
        Ok(QueueDirective::Wait(delay))
    }

    fn mark_rate_limited_paused(
        &mut self,
        job_id: &JobId,
        delay: QueueDelay,
    ) -> JobResult<QueueDirective> {
        let max_retries = self.retry_policy.max_rate_limit_retries;
        let batch = self.active_batch_mut()?;
        ensure_current_attempt_job(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        if job.retry_attempts >= max_retries {
            job.status = JobStatus::Blocked;
            batch.current_job = Some(job_id.clone());
            batch.batch.status = BatchStatus::Paused;
            batch.pause_after_current = false;
            return Ok(QueueDirective::Paused);
        }
        job.retry_attempts += 1;
        job.status = JobStatus::WaitingRetry;
        batch.current_job = Some(job_id.clone());
        batch.paused_delay = Some(delay);
        batch.pending_delay = None;
        batch.batch.status = BatchStatus::Paused;
        batch.pause_after_current = false;
        Ok(QueueDirective::Paused)
    }

    fn mark_current_failed(&mut self, job_id: &JobId) -> JobResult<()> {
        let batch = self.active_batch_mut()?;
        ensure_current_attempt_job(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        job.status = JobStatus::Failed;
        batch.current_job = None;
        Ok(())
    }

    fn block_current_job(&mut self, job_id: &JobId) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        ensure_current_attempt_job(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        job.status = JobStatus::Blocked;
        batch.current_job = Some(job_id.clone());
        batch.batch.status = BatchStatus::Paused;
        batch.pause_after_current = false;
        Ok(QueueDirective::Paused)
    }

    fn schedule_after_completed_job(&mut self) -> JobResult<QueueDirective> {
        let delay = self.retry_policy.task_interval;
        let batch = self.active_batch_mut()?;
        batch.current_job = None;
        if next_queued_job(&batch.batch.jobs).is_some() {
            batch.pending_delay = Some(delay);
            batch.batch.status = BatchStatus::Waiting;
            Ok(QueueDirective::Wait(delay))
        } else {
            batch.batch.status = BatchStatus::Succeeded;
            Ok(QueueDirective::Idle)
        }
    }

    fn pause_after_completed_job(&mut self) -> JobResult<QueueDirective> {
        let delay = self.retry_policy.task_interval;
        let batch = self.active_batch_mut()?;
        batch.pause_after_current = false;
        batch.current_job = None;
        if next_queued_job(&batch.batch.jobs).is_some() {
            batch.paused_delay = Some(delay);
            batch.batch.status = BatchStatus::Paused;
            Ok(QueueDirective::Paused)
        } else {
            batch.batch.status = BatchStatus::Succeeded;
            Ok(QueueDirective::Idle)
        }
    }

    fn finish_stopped_batch(&mut self) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        for job in &mut batch.batch.jobs {
            if !job.status.is_terminal() {
                job.status = JobStatus::Skipped;
            }
        }
        batch.current_job = None;
        batch.pending_delay = None;
        batch.paused_delay = None;
        batch.pause_after_current = false;
        batch.stop_after_current = false;
        batch.batch.status = BatchStatus::Stopped;
        Ok(QueueDirective::Idle)
    }
}

fn has_duplicate_job_ids(jobs: &[SubmitJob]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    jobs.iter().any(|job| !seen.insert(job.job_id.clone()))
}

fn find_job_mut<'a>(batch: &'a mut ActiveBatch, job_id: &JobId) -> JobResult<&'a mut JobRecord> {
    batch
        .batch
        .jobs
        .iter_mut()
        .find(|job| &job.job_id == job_id)
        .ok_or_else(|| JobQueueError::not_found("job does not exist in active batch"))
}

fn ensure_job_can_start(batch: &ActiveBatch, job_id: &JobId) -> JobResult<()> {
    if batch.batch.status != BatchStatus::Running
        || batch.pending_delay.is_some()
        || batch.paused_delay.is_some()
    {
        return Err(JobQueueError::invalid_state(
            "job cannot start while queue is waiting or paused",
        ));
    }
    if let Some(current_job) = &batch.current_job {
        if current_job == job_id {
            return Ok(());
        }
        return Err(JobQueueError::invalid_state(
            "only the current scheduled job can start",
        ));
    }
    let expected = retry_waiting_job(&batch.batch.jobs)
        .or_else(|| blocked_job(&batch.batch.jobs))
        .or_else(|| next_queued_job(&batch.batch.jobs));
    if expected.as_ref() == Some(job_id) {
        Ok(())
    } else {
        Err(JobQueueError::invalid_state(
            "job is not the next scheduled queue item",
        ))
    }
}

fn ensure_current_attempt_job(batch: &ActiveBatch, job_id: &JobId) -> JobResult<()> {
    if batch.current_job.as_ref() != Some(job_id) {
        return Err(JobQueueError::invalid_state(
            "failure does not belong to the current job",
        ));
    }
    let job = find_job(batch, job_id)?;
    if matches!(job.status, JobStatus::Preparing | JobStatus::Running) {
        Ok(())
    } else {
        Err(JobQueueError::invalid_state(
            "only preparing or running jobs can fail",
        ))
    }
}

fn find_job<'a>(batch: &'a ActiveBatch, job_id: &JobId) -> JobResult<&'a JobRecord> {
    batch
        .batch
        .jobs
        .iter()
        .find(|job| &job.job_id == job_id)
        .ok_or_else(|| JobQueueError::not_found("job does not exist in active batch"))
}

fn next_queued_job(jobs: &[JobRecord]) -> Option<JobId> {
    jobs.iter()
        .find(|job| job.status == JobStatus::Queued)
        .map(|job| job.job_id.clone())
}

fn retry_waiting_job(jobs: &[JobRecord]) -> Option<JobId> {
    jobs.iter()
        .find(|job| job.status == JobStatus::WaitingRetry)
        .map(|job| job.job_id.clone())
}

fn blocked_job(jobs: &[JobRecord]) -> Option<JobId> {
    jobs.iter()
        .find(|job| job.status == JobStatus::Blocked)
        .map(|job| job.job_id.clone())
}

fn has_running_job(jobs: &[JobRecord]) -> bool {
    jobs.iter()
        .any(|job| matches!(job.status, JobStatus::Preparing | JobStatus::Running))
}
