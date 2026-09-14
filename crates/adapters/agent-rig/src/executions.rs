use std::sync::{Arc, Mutex};

use atelier_agent::{
    AgentError, AgentResult, AgentRunObserver, AgentRuntimeEvent, AgentToolExecutor,
    AgentToolOutput,
};
use tokio::{sync::oneshot, task::JoinHandle};

/// Tools finish their persistence boundary even when model streaming is stopped.
/// The runtime drains these tasks before the app cancels owned generation work.
#[derive(Default)]
pub struct ToolExecutions(Mutex<Vec<JoinHandle<()>>>);

impl ToolExecutions {
    pub fn start(
        &self,
        executor: Arc<dyn AgentToolExecutor>,
        observer: Arc<dyn AgentRunObserver>,
        call_id: String,
        name: String,
        arguments: String,
    ) -> oneshot::Receiver<AgentResult<AgentToolOutput>> {
        let (sender, receiver) = oneshot::channel();
        let task = tokio::spawn(async move {
            let result = executor.execute(&call_id, &name, &arguments).await;
            observer.emit(AgentRuntimeEvent::ToolFinished {
                name,
                result_json: result
                    .as_ref()
                    .map_or_else(ToString::to_string, |value| value.text.clone()),
                failed: result.is_err(),
            });
            let _ = sender.send(result);
        });
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(task);
        receiver
    }

    pub async fn drain(&self) -> AgentResult<()> {
        let tasks = std::mem::take(
            &mut *self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
        for result in futures_util::future::join_all(tasks).await {
            result
                .map_err(|error| AgentError::runtime(format!("tool execution failed: {error}")))?;
        }
        Ok(())
    }
}
