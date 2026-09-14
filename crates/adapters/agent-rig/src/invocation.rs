use std::sync::{Arc, Mutex};

use atelier_agent::{AgentError, AgentResult, AgentToolExecutor};
use rig_agent::agent::{
    AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, ToolCall, ToolCallAction,
};
use serde_json::Value;

/// Rig dispatch has no call identity in `ToolContext`. With `tool_concurrency(1)`,
/// the hook and callback exchange it here without modifying model arguments.
#[derive(Default)]
pub struct InvocationSlot(Mutex<Option<Invocation>>);

struct Invocation {
    id: String,
    name: String,
    arguments: Value,
}

impl InvocationSlot {
    fn publish(&self, id: &str, name: &str, arguments: Value) -> AgentResult<()> {
        let mut slot = self.0.lock().map_err(|_| unavailable())?;
        if slot.is_some() {
            return Err(AgentError::runtime(
                "overlapping tool dispatch is unsupported",
            ));
        }
        *slot = Some(Invocation {
            id: id.into(),
            name: name.into(),
            arguments,
        });
        drop(slot);
        Ok(())
    }

    pub(super) fn take(&self, name: &str, arguments: &Value) -> AgentResult<String> {
        let call = self
            .0
            .lock()
            .map_err(|_| unavailable())?
            .take()
            .ok_or_else(unavailable)?;
        if call.name != name || call.arguments != *arguments {
            return Err(AgentError::runtime(
                "tool dispatch does not match its host identity",
            ));
        }
        Ok(call.id)
    }
}

fn unavailable() -> AgentError {
    AgentError::runtime("tool invocation identity is unavailable")
}

pub struct RuntimeHook {
    pub executor: Arc<dyn AgentToolExecutor>,
    pub invocation: Arc<InvocationSlot>,
    pub budget: super::context_budget::ContextBudget,
}

impl AgentHook for RuntimeHook {
    async fn on_completion_call(
        &self,
        _: &HookContext,
        event: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        match self.executor.begin_model_step().await {
            Ok(()) => match self.budget.prepare(event.history, event.prompt) {
                Ok(history) => CompletionCallAction::patch(rig_agent::agent::RequestPatch {
                    history: Some(history),
                    ..Default::default()
                }),
                Err(error) => CompletionCallAction::stop(error.to_string()),
            },
            Err(error) => CompletionCallAction::stop(error.to_string()),
        }
    }

    fn on_tool_call(
        &self,
        _: &HookContext,
        event: ToolCall<'_>,
    ) -> impl std::future::Future<Output = ToolCallAction> {
        let result = serde_json::from_str(event.args)
            .map_err(|error| AgentError::validation(error.to_string()))
            .and_then(|arguments| {
                self.invocation
                    .publish(event.internal_call_id, event.tool_name, arguments)
            });
        std::future::ready(match result {
            Ok(()) => ToolCallAction::Run,
            Err(error) => ToolCallAction::Stop(error.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn same_arguments_can_have_distinct_host_identities() {
        let slot = InvocationSlot::default();
        for id in ["first", "second"] {
            slot.publish(id, "submit", json!({})).unwrap();
            assert_eq!(slot.take("submit", &json!({})).unwrap(), id);
        }
        assert!(slot.take("submit", &json!({})).is_err());
    }

    #[test]
    fn overlapping_or_mismatched_dispatch_is_rejected() {
        let slot = InvocationSlot::default();
        slot.publish("one", "edit", json!({"text":"a"})).unwrap();
        assert!(slot.publish("two", "edit", json!({})).is_err());
        assert!(slot.take("edit", &json!({"text":"b"})).is_err());
        slot.publish("three", "edit", json!({})).unwrap();
        assert!(slot.take("submit", &json!({})).is_err());
    }
}
