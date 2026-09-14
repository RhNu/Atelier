use std::sync::Mutex;

use atelier_agent::{AgentError, AgentResult};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

struct ActiveTurn {
    session_id: String,
    cancellation: CancellationToken,
    approval: Option<PendingApproval>,
}

struct PendingApproval {
    id: String,
    decision: oneshot::Sender<bool>,
}

#[derive(Default)]
pub struct AgentTurnCoordinator {
    active: Mutex<Option<ActiveTurn>>,
    pub(crate) generations: super::generation_control::GenerationControls,
}

impl AgentTurnCoordinator {
    pub fn begin(&self, session_id: &str) -> AgentResult<CancellationToken> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| AgentError::runtime("agent turn state is unavailable"))?;
        if active.is_some() {
            return Err(AgentError::conflict(
                "another Agent turn is already active in this workspace",
            ));
        }
        let cancellation = CancellationToken::new();
        *active = Some(ActiveTurn {
            session_id: session_id.to_owned(),
            cancellation: cancellation.clone(),
            approval: None,
        });
        drop(active);
        Ok(cancellation)
    }

    pub fn finish(&self, session_id: &str) {
        if let Ok(mut active) = self.active.lock()
            && active
                .as_ref()
                .is_some_and(|value| value.session_id == session_id)
        {
            *active = None;
        }
    }

    pub fn cancel(&self, session_id: Option<&str>) -> AgentResult<bool> {
        if session_id.is_none() {
            self.generations.cancel_all()?;
        }
        let active_guard = self
            .active
            .lock()
            .map_err(|_| AgentError::runtime("agent turn state is unavailable"))?;
        let Some(active) = active_guard.as_ref() else {
            return Ok(false);
        };
        if session_id.is_some_and(|id| id != active.session_id) {
            return Ok(false);
        }
        let cancellation = active.cancellation.clone();
        drop(active_guard);
        cancellation.cancel();
        Ok(true)
    }

    pub fn register_approval(
        &self,
        session_id: &str,
        approval_id: String,
    ) -> AgentResult<oneshot::Receiver<bool>> {
        let mut active_guard = self
            .active
            .lock()
            .map_err(|_| AgentError::runtime("agent turn state is unavailable"))?;
        let active = active_guard
            .as_mut()
            .filter(|value| value.session_id == session_id)
            .ok_or_else(|| AgentError::conflict("the Agent turn is no longer active"))?;
        if active.approval.is_some() {
            return Err(AgentError::conflict(
                "another Agent tool approval is already pending",
            ));
        }
        let (decision, receiver) = oneshot::channel();
        active.approval = Some(PendingApproval {
            id: approval_id,
            decision,
        });
        drop(active_guard);
        Ok(receiver)
    }

    pub fn decide(&self, approval_id: &str, approved: bool) -> AgentResult<()> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| AgentError::runtime("agent turn state is unavailable"))?;
        let pending = active
            .as_mut()
            .and_then(|value| value.approval.take())
            .ok_or_else(|| AgentError::not_found("agent approval does not exist"))?;
        if pending.id != approval_id {
            if let Some(active) = active.as_mut() {
                active.approval = Some(pending);
            }
            return Err(AgentError::not_found("agent approval does not exist"));
        }
        drop(active);
        pending
            .decision
            .send(approved)
            .map_err(|_| AgentError::conflict("agent approval is no longer pending"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_turn_can_be_active() {
        let coordinator = AgentTurnCoordinator::default();
        coordinator.begin("one").unwrap();
        assert_eq!(
            coordinator.begin("two").expect_err("conflict").kind,
            atelier_agent::AgentErrorKind::Conflict
        );
        assert!(!coordinator.cancel(Some("two")).unwrap());
        assert!(coordinator.cancel(Some("one")).unwrap());
    }
}
