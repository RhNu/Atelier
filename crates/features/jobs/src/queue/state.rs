use super::{
    ActiveJobBatchSnapshot, BatchStatus, JobBatch, JobId, JobQueueError, JobRecord, JobResult,
    JobStatus, QueueDelay, SubmitJob,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ActiveBatch {
    pub(super) batch: JobBatch,
    pub(super) current_job: Option<JobId>,
    pub(super) pending_delay: Option<QueueDelay>,
    pub(super) paused_delay: Option<QueueDelay>,
    pub(super) pause_after_current: bool,
    pub(super) stop_after_current: bool,
}

impl ActiveBatch {
    pub(super) fn snapshot(&self) -> ActiveJobBatchSnapshot {
        ActiveJobBatchSnapshot {
            batch: self.batch.clone(),
            current_job: self.current_job.clone(),
            pending_delay: self.pending_delay,
            paused_delay: self.paused_delay,
            pause_after_current: self.pause_after_current,
            stop_after_current: self.stop_after_current,
        }
    }

    pub(super) fn from_snapshot(snapshot: ActiveJobBatchSnapshot) -> Self {
        Self {
            batch: snapshot.batch,
            current_job: snapshot.current_job,
            pending_delay: snapshot.pending_delay,
            paused_delay: snapshot.paused_delay,
            pause_after_current: snapshot.pause_after_current,
            stop_after_current: snapshot.stop_after_current,
        }
    }
}

pub(super) fn has_duplicate_job_ids(jobs: &[SubmitJob]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    jobs.iter().any(|job| !seen.insert(job.job_id.clone()))
}

pub(super) fn has_duplicate_job_records(jobs: &[JobRecord]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    jobs.iter().any(|job| !seen.insert(job.job_id.clone()))
}

pub(super) fn find_job_mut<'a>(
    batch: &'a mut ActiveBatch,
    job_id: &JobId,
) -> JobResult<&'a mut JobRecord> {
    batch
        .batch
        .jobs
        .iter_mut()
        .find(|job| &job.job_id == job_id)
        .ok_or_else(|| JobQueueError::not_found("job does not exist in active batch"))
}

pub(super) fn ensure_job_can_start(batch: &ActiveBatch, job_id: &JobId) -> JobResult<()> {
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

pub(super) fn ensure_current_attempt_job(batch: &ActiveBatch, job_id: &JobId) -> JobResult<()> {
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

pub(super) fn find_job<'a>(batch: &'a ActiveBatch, job_id: &JobId) -> JobResult<&'a JobRecord> {
    batch
        .batch
        .jobs
        .iter()
        .find(|job| &job.job_id == job_id)
        .ok_or_else(|| JobQueueError::not_found("job does not exist in active batch"))
}

pub(super) fn next_queued_job(jobs: &[JobRecord]) -> Option<JobId> {
    jobs.iter()
        .find(|job| job.status == JobStatus::Queued)
        .map(|job| job.job_id.clone())
}

pub(super) fn retry_waiting_job(jobs: &[JobRecord]) -> Option<JobId> {
    jobs.iter()
        .find(|job| job.status == JobStatus::WaitingRetry)
        .map(|job| job.job_id.clone())
}

pub(super) fn blocked_job(jobs: &[JobRecord]) -> Option<JobId> {
    jobs.iter()
        .find(|job| job.status == JobStatus::Blocked)
        .map(|job| job.job_id.clone())
}

pub(super) fn has_running_job(jobs: &[JobRecord]) -> bool {
    jobs.iter()
        .any(|job| matches!(job.status, JobStatus::Preparing | JobStatus::Running))
}
