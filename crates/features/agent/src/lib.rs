//! Domain contracts for Atelier's `NovelAI` workflow agent.

mod error;
mod model;
mod observation;
mod ports;
mod prompt;
mod runtime;
mod service;

pub use error::{AgentError, AgentErrorKind, AgentResult};
pub use model::{
    AgentAction, AgentActionId, AgentActionState, AgentAuth, AgentConnection, AgentConnectionId,
    AgentEvent, AgentEventId, AgentEventKind, AgentImageInputMode, AgentModel,
    AgentModelCapabilities, AgentModelId, AgentModelSnapshot, AgentPermissionMode,
    AgentPersonaSnapshot, AgentProbeStatus, AgentProtocol, AgentRegistry, AgentSession,
    AgentSessionId, AgentSessionStatus, AgentSummary, AgentWorkspaceSettings,
};
pub use observation::AgentObservation;
pub use ports::{AgentRegistryRepository, AgentWorkspaceRepository};
pub use prompt::{AgentPromptFamily, build_agent_system_prompt};
pub use runtime::*;
pub use service::{AgentRegistryService, AgentWorkspaceService};

mod tool_output;
pub use tool_output::{AgentImageMediaType, AgentToolImage, AgentToolOutput};
