use std::sync::{Arc, Mutex, MutexGuard};

use atelier_jobs::{JobQueue, JobQueueSnapshot};

/// Read access to generation state without acquiring the execution lock.
#[derive(Clone, Debug)]
pub struct QueueView(Arc<Mutex<JobQueue>>);

impl QueueView {
    pub(crate) fn new(queue: JobQueue) -> Self {
        Self(Arc::new(Mutex::new(queue)))
    }

    #[must_use]
    pub fn snapshot(&self) -> JobQueueSnapshot {
        self.lock().snapshot()
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, JobQueue> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
