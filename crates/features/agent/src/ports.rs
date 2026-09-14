use async_trait::async_trait;

use crate::{
    AgentAction, AgentActionId, AgentEvent, AgentRegistry, AgentResult, AgentSession,
    AgentSessionId, AgentSummary, AgentWorkspaceSettings,
};

#[async_trait]
pub trait AgentRegistryRepository: Send + Sync {
    async fn load_registry(&self) -> AgentResult<AgentRegistry>;
    async fn save_registry(&self, registry: AgentRegistry) -> AgentResult<()>;
}

#[async_trait]
pub trait AgentWorkspaceRepository: Send + Sync {
    async fn get_settings(&self) -> AgentResult<AgentWorkspaceSettings>;
    async fn save_settings(&self, settings: AgentWorkspaceSettings) -> AgentResult<()>;
    async fn list_sessions(&self) -> AgentResult<Vec<AgentSession>>;
    async fn get_session(&self, id: &AgentSessionId) -> AgentResult<Option<AgentSession>>;
    async fn save_session(&self, session: AgentSession) -> AgentResult<()>;
    async fn delete_session(&self, id: &AgentSessionId) -> AgentResult<bool>;
    async fn list_events(&self, session_id: &AgentSessionId) -> AgentResult<Vec<AgentEvent>>;
    async fn append_event(&self, event: AgentEvent) -> AgentResult<()>;
    async fn get_action(&self, id: &AgentActionId) -> AgentResult<Option<AgentAction>>;
    async fn save_action(&self, action: AgentAction) -> AgentResult<()>;
    async fn get_summary(&self, session_id: &AgentSessionId) -> AgentResult<Option<AgentSummary>>;
    async fn save_summary(&self, summary: AgentSummary) -> AgentResult<()>;
    async fn interrupt_running_sessions(&self, updated_at_ms: u64) -> AgentResult<usize>;
}
