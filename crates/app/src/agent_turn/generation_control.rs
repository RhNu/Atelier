use std::{collections::BTreeMap, sync::Mutex};

use atelier_agent::{AgentError, AgentResult};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct GenerationControls(Mutex<BTreeMap<String, OwnedBatch>>);

struct OwnedBatch {
    jobs: Vec<String>,
    cancellation: CancellationToken,
}

impl GenerationControls {
    pub fn register(
        &self,
        batch: String,
        jobs: Vec<String>,
        parent: &CancellationToken,
    ) -> AgentResult<()> {
        self.0.lock().map_err(|_| unavailable())?.insert(
            batch,
            OwnedBatch {
                jobs,
                cancellation: parent.child_token(),
            },
        );
        Ok(())
    }

    pub fn for_job(&self, job: &str) -> AgentResult<Option<CancellationToken>> {
        Ok(self
            .0
            .lock()
            .map_err(|_| unavailable())?
            .values()
            .find(|batch| batch.jobs.iter().any(|id| id == job))
            .map(|batch| batch.cancellation.clone()))
    }

    pub fn cancel(&self, batch: &str) -> AgentResult<()> {
        let batches = self.0.lock().map_err(|_| unavailable())?;
        if let Some(batch) = batches.get(batch) {
            batch.cancellation.cancel();
        }
        drop(batches);
        Ok(())
    }

    pub fn cancel_all(&self) -> AgentResult<()> {
        for batch in self.0.lock().map_err(|_| unavailable())?.values() {
            batch.cancellation.cancel();
        }
        Ok(())
    }

    pub fn remove(&self, batch: &str) -> AgentResult<()> {
        self.0.lock().map_err(|_| unavailable())?.remove(batch);
        Ok(())
    }
}

fn unavailable() -> AgentError {
    AgentError::runtime("Agent generation controls unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_one_batch_does_not_cancel_another_or_the_turn() {
        let controls = GenerationControls::default();
        let turn = CancellationToken::new();
        controls
            .register("a".into(), vec!["job-a".into()], &turn)
            .unwrap();
        controls
            .register("b".into(), vec!["job-b".into()], &turn)
            .unwrap();
        controls.cancel("a").unwrap();
        assert!(controls.for_job("job-a").unwrap().unwrap().is_cancelled());
        assert!(!controls.for_job("job-b").unwrap().unwrap().is_cancelled());
        assert!(!turn.is_cancelled());
        assert!(controls.for_job("user-job").unwrap().is_none());
        turn.cancel();
        assert!(controls.for_job("job-b").unwrap().unwrap().is_cancelled());
    }
}
