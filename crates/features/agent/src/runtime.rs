use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use crate::{AgentModelSnapshot, AgentResult};

#[derive(Clone)]
pub struct AgentResolvedConnection {
    pub base_url: String,
    pub bearer_token: Option<String>,
}

impl std::fmt::Debug for AgentResolvedConnection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentResolvedConnection")
            .field("base_url", &self.base_url)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AgentChatRole {
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentChatMessage {
    pub role: AgentChatRole,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_json: String,
}

#[derive(Clone, Debug)]
pub struct AgentRunRequest {
    pub connection: AgentResolvedConnection,
    pub model: AgentModelSnapshot,
    pub system_prompt: String,
    pub user_message: String,
    pub history: Vec<AgentChatMessage>,
    pub tools: Vec<AgentToolSpec>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentRuntimeEvent {
    AssistantTextDelta {
        text: String,
    },
    ToolStarted {
        name: String,
        arguments_json: String,
    },
    ToolFinished {
        name: String,
        result_json: String,
        failed: bool,
    },
    ApprovalRequested {
        approval_id: String,
        name: String,
        arguments_json: String,
    },
    ApprovalResolved {
        approval_id: String,
        approved: bool,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
}

pub trait AgentRunObserver: Send + Sync {
    fn emit(&self, event: AgentRuntimeEvent);
}

impl<F> AgentRunObserver for F
where
    F: Fn(AgentRuntimeEvent) + Send + Sync,
{
    fn emit(&self, event: AgentRuntimeEvent) {
        self(event);
    }
}

#[async_trait]
pub trait AgentToolExecutor: Send + Sync {
    async fn execute(&self, tool_name: &str, arguments_json: &str) -> AgentResult<String>;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentRunOutcome {
    pub assistant_message: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub model_calls: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredAgentModel {
    pub wire_model_id: String,
    pub display_name: String,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

#[async_trait]
pub trait AgentModelRuntime: Send + Sync {
    async fn discover_models(
        &self,
        connection: AgentResolvedConnection,
    ) -> AgentResult<Vec<DiscoveredAgentModel>>;

    async fn run(
        &self,
        request: AgentRunRequest,
        tools: Arc<dyn AgentToolExecutor>,
        observer: Arc<dyn AgentRunObserver>,
        cancellation: CancellationToken,
    ) -> AgentResult<AgentRunOutcome>;
}

#[derive(Clone, Debug, Default)]
pub struct UnavailableAgentModelRuntime;

#[async_trait]
impl AgentModelRuntime for UnavailableAgentModelRuntime {
    async fn discover_models(
        &self,
        _connection: AgentResolvedConnection,
    ) -> AgentResult<Vec<DiscoveredAgentModel>> {
        Err(crate::AgentError::runtime(
            "agent model runtime is unavailable",
        ))
    }

    async fn run(
        &self,
        _request: AgentRunRequest,
        _tools: Arc<dyn AgentToolExecutor>,
        _observer: Arc<dyn AgentRunObserver>,
        _cancellation: CancellationToken,
    ) -> AgentResult<AgentRunOutcome> {
        Err(crate::AgentError::runtime(
            "agent model runtime is unavailable",
        ))
    }
}
