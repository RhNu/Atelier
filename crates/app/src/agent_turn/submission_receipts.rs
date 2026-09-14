use std::collections::BTreeMap;

use atelier_agent::{AgentError, AgentResult};
use serde_json::Value;

#[derive(Default)]
pub(super) struct SubmissionReceipts(BTreeMap<String, Attempt>);

struct Attempt {
    arguments: Value,
    result: Option<AgentResult<String>>,
}

impl SubmissionReceipts {
    /// Reserves a host invocation before approval or I/O. An interrupted attempt
    /// is never silently re-executed: its outcome may already be durable.
    pub fn begin(
        &mut self,
        id: &str,
        arguments: Value,
    ) -> AgentResult<Option<AgentResult<String>>> {
        if let Some(attempt) = self.0.get(id) {
            if attempt.arguments != arguments {
                return Err(AgentError::conflict(
                    "submission retry changed its arguments",
                ));
            }
            return attempt.result.clone().map(Some).ok_or_else(|| {
                AgentError::conflict("submission is in progress or its outcome is unknown; inspect generation status")
            });
        }
        self.0.insert(
            id.into(),
            Attempt {
                arguments,
                result: None,
            },
        );
        Ok(None)
    }

    pub fn finish(&mut self, id: &str, result: AgentResult<String>) -> AgentResult<()> {
        let attempt = self
            .0
            .get_mut(id)
            .ok_or_else(|| AgentError::runtime("submission identity missing"))?;
        attempt.result = Some(result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn retry_returns_original_receipt_but_new_invocation_can_submit_again() {
        let mut receipts = SubmissionReceipts::default();
        assert_eq!(receipts.begin("a", json!({})).unwrap(), None);
        receipts.finish("a", Ok("batch-a".into())).unwrap();
        assert_eq!(
            receipts.begin("a", json!({})).unwrap(),
            Some(Ok("batch-a".into()))
        );
        assert_eq!(receipts.begin("b", json!({})).unwrap(), None);
    }

    #[test]
    fn unknown_or_changed_attempt_cannot_repeat_side_effects() {
        let mut receipts = SubmissionReceipts::default();
        receipts.begin("a", json!({})).unwrap();
        assert!(receipts.begin("a", json!({})).is_err());
        assert!(receipts.begin("a", json!({"changed":true})).is_err());
        let denied = Err(AgentError::conflict("denied"));
        receipts.finish("a", denied.clone()).unwrap();
        assert_eq!(receipts.begin("a", json!({})).unwrap(), Some(denied));
    }
}
