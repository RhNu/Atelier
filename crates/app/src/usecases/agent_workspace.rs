use atelier_adapter_database::DatabaseAgentWorkspaceRepository;
use atelier_agent::{AgentModelId, AgentRegistry, AgentSessionId, AgentWorkspaceService};
use atelier_app_api::agent::{
    AgentEventDto, AgentSessionDto, AgentWorkspaceSettingsDto, CreateAgentSessionRequestDto,
    RenameAgentSessionRequestDto, UpdateAgentWorkspaceSettingsRequestDto,
};

use crate::mapping::{
    agent_event_to_dto, agent_session_to_dto, agent_workspace_settings_to_domain,
    agent_workspace_settings_to_dto,
};
use crate::{AppError, AppResult};

pub struct AgentWorkspaceUseCases<'a> {
    pub(crate) turns: &'a std::sync::Arc<crate::agent_turn::AgentTurnCoordinator>,
    pub(crate) agent: &'a AgentWorkspaceService<DatabaseAgentWorkspaceRepository>,
}

impl AgentWorkspaceUseCases<'_> {
    /// Returns workspace Agent personalization and permission settings.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn get_settings(&self) -> AppResult<AgentWorkspaceSettingsDto> {
        self.agent
            .get_settings()
            .await
            .map(|value| agent_workspace_settings_to_dto(&value))
            .map_err(AppError::from)
    }

    /// Replaces workspace Agent personalization and permission settings.
    ///
    /// # Errors
    /// Returns an error when validation or persistence fails.
    pub async fn update_settings(
        &self,
        request: UpdateAgentWorkspaceSettingsRequestDto,
    ) -> AppResult<AgentWorkspaceSettingsDto> {
        let previous = self.agent.get_settings().await?;
        let saved = self
            .agent
            .update_settings(agent_workspace_settings_to_domain(request.settings))
            .await?;
        if previous.output_vision_enabled && !saved.output_vision_enabled {
            self.turns.cancel(None)?;
        }
        Ok(agent_workspace_settings_to_dto(&saved))
    }

    /// Creates a session with immutable model and persona snapshots.
    ///
    /// # Errors
    /// Returns an error when the model is missing or persistence fails.
    pub async fn create_session(
        &self,
        request: CreateAgentSessionRequestDto,
        registry: &AgentRegistry,
    ) -> AppResult<AgentSessionDto> {
        self.agent
            .create_session(
                AgentSessionId::new(uuid::Uuid::new_v4().to_string()),
                request.title,
                &AgentModelId::new(request.model_id),
                registry,
                crate::time::unix_timestamp_ms(),
            )
            .await
            .map(|value| agent_session_to_dto(&value))
            .map_err(AppError::from)
    }

    /// Lists sessions ordered by most recent activity.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn list_sessions(&self) -> AppResult<Vec<AgentSessionDto>> {
        self.agent
            .list_sessions()
            .await
            .map(|items| items.iter().map(agent_session_to_dto).collect())
            .map_err(AppError::from)
    }

    /// Renames an inactive session.
    ///
    /// # Errors
    /// Returns an error when the session is active, missing, invalid, or cannot be persisted.
    pub async fn rename_session(
        &self,
        request: RenameAgentSessionRequestDto,
    ) -> AppResult<AgentSessionDto> {
        self.agent
            .rename_session(
                &AgentSessionId::new(request.session_id),
                request.title,
                crate::time::unix_timestamp_ms(),
            )
            .await
            .map(|value| agent_session_to_dto(&value))
            .map_err(AppError::from)
    }

    /// Deletes an inactive session and all of its owned records.
    ///
    /// # Errors
    /// Returns an error when it is active or persistence fails.
    pub async fn delete_session(&self, id: &str) -> AppResult<bool> {
        self.agent
            .delete_session(&AgentSessionId::new(id))
            .await
            .map_err(AppError::from)
    }

    /// Returns a session's complete typed event log.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn list_events(&self, id: &str) -> AppResult<Vec<AgentEventDto>> {
        self.agent
            .list_events(&AgentSessionId::new(id))
            .await
            .map(|items| items.iter().map(agent_event_to_dto).collect())
            .map_err(AppError::from)
    }
}
