use crate::{BatchId, JobQueue, JobResult};

impl JobQueue {
    /// Stops the matching batch after its executor has relinquished the work.
    /// Completed outputs keep their status; unfinished requests become skipped.
    ///
    /// # Errors
    /// Returns an error if the queue cannot complete the stop transition.
    pub fn cancel_batch(&mut self, id: &BatchId) -> JobResult<bool> {
        if !self.active_batch.as_ref().is_some_and(|active| {
            &active.batch.batch_id == id && !active.batch.status.is_terminal()
        }) {
            return Ok(false);
        }
        self.finish_stopped_batch()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JobId, JobKind, JobPayloadRef, JobStatus, SubmitJob};

    #[test]
    fn cancellation_is_scoped_and_terminal() {
        let mut queue = JobQueue::default();
        let batch = BatchId::new("agent");
        let jobs = ["a", "b"].map(|id| SubmitJob {
            job_id: JobId::new(id),
            kind: JobKind::GenerateImage,
            payload_ref: JobPayloadRef::new(id),
        });
        queue.submit_batch(batch.clone(), jobs.to_vec()).unwrap();
        queue.mark_preparing(&JobId::new("a")).unwrap();
        queue
            .mark_running(&JobId::new("a"), JobPayloadRef::new("prepared"))
            .unwrap();
        let before = queue.snapshot();
        assert!(!queue.cancel_batch(&BatchId::new("user")).unwrap());
        assert_eq!(queue.snapshot(), before);
        assert!(queue.cancel_batch(&batch).unwrap());
        assert_eq!(queue.job_status(&JobId::new("a")), Some(JobStatus::Skipped));
        assert_eq!(queue.job_status(&JobId::new("b")), Some(JobStatus::Skipped));
        assert!(!queue.cancel_batch(&batch).unwrap());
    }

    #[test]
    fn cancelling_pending_work_preserves_completed_requests() {
        let mut queue = JobQueue::default();
        let batch = BatchId::new("batch");
        let jobs = ["a", "b"].map(|id| SubmitJob {
            job_id: JobId::new(id),
            kind: JobKind::GenerateImage,
            payload_ref: JobPayloadRef::new(id),
        });
        queue.submit_batch(batch.clone(), jobs.to_vec()).unwrap();
        queue.mark_preparing(&JobId::new("a")).unwrap();
        queue
            .mark_running(&JobId::new("a"), JobPayloadRef::new("prepared"))
            .unwrap();
        queue.mark_succeeded(&JobId::new("a")).unwrap();
        assert!(queue.cancel_batch(&batch).unwrap());
        assert_eq!(
            queue.job_status(&JobId::new("a")),
            Some(JobStatus::Succeeded)
        );
        assert_eq!(queue.job_status(&JobId::new("b")), Some(JobStatus::Skipped));
    }
}
