//! Domain contracts for Atelier's `NovelAI` workflow agent.

mod error;
mod model;
mod ports;
mod service;

pub use error::{AgentError, AgentErrorKind, AgentResult};
pub use model::{
    AgentAction, AgentActionId, AgentActionState, AgentAuth, AgentConnection, AgentConnectionId,
    AgentEvent, AgentEventId, AgentEventKind, AgentModel, AgentModelId, AgentModelSnapshot,
    AgentPermissionMode, AgentPersonaSnapshot, AgentProbeStatus, AgentRegistry, AgentSession,
    AgentSessionId, AgentSessionStatus, AgentSummary, AgentWorkspaceSettings,
};
pub use ports::{AgentRegistryRepository, AgentWorkspaceRepository};
pub use service::{AgentRegistryService, AgentWorkspaceService};
