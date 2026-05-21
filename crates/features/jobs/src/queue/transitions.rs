use super::{
    BatchStatus, JobId, JobQueue, JobResult, JobStatus, QueueDelay, QueueDirective,
    ensure_current_attempt_job, find_job_mut, next_queued_job,
};

impl JobQueue {
    pub(super) fn mark_rate_limited(
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

    pub(super) fn mark_rate_limited_paused(
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

    pub(super) fn mark_current_failed(&mut self, job_id: &JobId) -> JobResult<()> {
        let batch = self.active_batch_mut()?;
        ensure_current_attempt_job(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        job.status = JobStatus::Failed;
        batch.current_job = None;
        Ok(())
    }

    pub(super) fn block_current_job(&mut self, job_id: &JobId) -> JobResult<QueueDirective> {
        let batch = self.active_batch_mut()?;
        ensure_current_attempt_job(batch, job_id)?;
        let job = find_job_mut(batch, job_id)?;
        job.status = JobStatus::Blocked;
        batch.current_job = Some(job_id.clone());
        batch.batch.status = BatchStatus::Paused;
        batch.pause_after_current = false;
        Ok(QueueDirective::Paused)
    }

    pub(super) fn schedule_after_completed_job(&mut self) -> JobResult<QueueDirective> {
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

    pub(super) fn pause_after_completed_job(&mut self) -> JobResult<QueueDirective> {
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

    pub(super) fn finish_stopped_batch(&mut self) -> JobResult<QueueDirective> {
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
