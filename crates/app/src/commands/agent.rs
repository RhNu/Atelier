use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::agent::{
    AgentEventDto, AgentRegistryDto, AgentSessionDto, AgentTurnEventDto, AgentTurnResultDto,
    AgentWorkspaceSettingsDto, CancelAgentTurnRequestDto, CreateAgentSessionRequestDto,
    DecideAgentApprovalRequestDto, DeleteAgentConnectionRequestDto, DeleteAgentModelRequestDto,
    DeleteAgentSessionRequestDto, DeleteAgentSessionResponseDto, DiscoverAgentModelsRequestDto,
    DiscoveredAgentModelDto, ListAgentEventsRequestDto, RenameAgentSessionRequestDto,
    RunAgentTurnRequestDto, SaveAgentConnectionRequestDto, SaveAgentModelRequestDto,
    UndoAgentActionRequestDto, UpdateAgentWorkspaceSettingsRequestDto,
};
use atelier_app_api::generation::VersionedGenerationDraftDto;
use atelier_secrets::SecretStore;

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    /// Returns application-global Agent model configuration.
    ///
    /// # Errors
    /// Returns an error envelope when registry persistence fails.
    pub async fn get_agent_registry(&self) -> CommandResult<AgentRegistryDto> {
        Self::command_result(self.agent_models().get_registry().await)
    }

    /// Discovers models from one configured connection's `/models` endpoint.
    ///
    /// # Errors
    /// Returns an error envelope when configuration, credentials, or the provider request fails.
    pub async fn discover_agent_models(
        &self,
        request: DiscoverAgentModelsRequestDto,
    ) -> CommandResult<Vec<DiscoveredAgentModelDto>> {
        Self::command_result(
            self.agent_models()
                .discover_models(&request.connection_id)
                .await,
        )
    }

    /// Creates or updates an Agent connection.
    ///
    /// # Errors
    /// Returns an error envelope when validation or persistence fails.
    pub async fn save_agent_connection(
        &self,
        request: SaveAgentConnectionRequestDto,
    ) -> CommandResult<AgentRegistryDto> {
        Self::command_result(self.agent_models().save_connection(request).await)
    }

    /// Deletes an Agent connection.
    ///
    /// # Errors
    /// Returns an error envelope when it is missing, referenced, or persistence fails.
    pub async fn delete_agent_connection(
        &self,
        request: DeleteAgentConnectionRequestDto,
    ) -> CommandResult<AgentRegistryDto> {
        Self::command_result(self.agent_models().delete_connection(&request.id).await)
    }

    /// Creates or updates an Agent model.
    ///
    /// # Errors
    /// Returns an error envelope when validation or persistence fails.
    pub async fn save_agent_model(
        &self,
        request: SaveAgentModelRequestDto,
    ) -> CommandResult<AgentRegistryDto> {
        Self::command_result(self.agent_models().save_model(request).await)
    }

    /// Deletes an Agent model.
    ///
    /// # Errors
    /// Returns an error envelope when it is missing or persistence fails.
    pub async fn delete_agent_model(
        &self,
        request: DeleteAgentModelRequestDto,
    ) -> CommandResult<AgentRegistryDto> {
        Self::command_result(self.agent_models().delete_model(&request.id).await)
    }

    /// Returns current workspace Agent settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or persistence fails.
    pub async fn get_agent_workspace_settings(&self) -> CommandResult<AgentWorkspaceSettingsDto> {
        let session = self.current_session()?;
        Self::command_result(session.agent().get_settings().await)
    }

    /// Replaces current workspace Agent settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, validation fails, or persistence fails.
    pub async fn update_agent_workspace_settings(
        &self,
        request: UpdateAgentWorkspaceSettingsRequestDto,
    ) -> CommandResult<AgentWorkspaceSettingsDto> {
        let session = self.current_session()?;
        Self::command_result(session.agent().update_settings(request).await)
    }

    /// Creates a durable workspace Agent session.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the model is missing, or persistence fails.
    pub async fn create_agent_session(
        &self,
        request: CreateAgentSessionRequestDto,
    ) -> CommandResult<AgentSessionDto> {
        let session = self.current_session()?;
        let registry = self
            .agent_registry
            .get_registry()
            .await
            .map_err(crate::AppError::from)
            .map_err(|error| error.envelope())?;
        Self::command_result(session.agent().create_session(request, &registry).await)
    }

    /// Lists durable workspace Agent sessions.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or persistence fails.
    pub async fn list_agent_sessions(&self) -> CommandResult<Vec<AgentSessionDto>> {
        let session = self.current_session()?;
        Self::command_result(session.agent().list_sessions().await)
    }

    /// Renames an inactive workspace Agent session.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the session is active, or persistence fails.
    pub async fn rename_agent_session(
        &self,
        request: RenameAgentSessionRequestDto,
    ) -> CommandResult<AgentSessionDto> {
        let session = self.current_session()?;
        Self::command_result(session.agent().rename_session(request).await)
    }

    /// Deletes an inactive workspace Agent session.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the session is active, or persistence fails.
    pub async fn delete_agent_session(
        &self,
        request: DeleteAgentSessionRequestDto,
    ) -> CommandResult<DeleteAgentSessionResponseDto> {
        let session = self.current_session()?;
        Self::command_result(
            session
                .agent()
                .delete_session(&request.session_id)
                .await
                .map(|deleted| DeleteAgentSessionResponseDto { deleted }),
        )
    }

    /// Returns a workspace Agent session's complete event log.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or persistence fails.
    pub async fn list_agent_events(
        &self,
        request: ListAgentEventsRequestDto,
    ) -> CommandResult<Vec<AgentEventDto>> {
        let session = self.current_session()?;
        Self::command_result(session.agent().list_events(&request.session_id).await)
    }

    /// Runs one streaming Agent turn against the current workspace.
    ///
    /// # Errors
    /// Returns an error envelope when the session, model, provider, or a tool operation fails.
    pub async fn run_agent_turn(
        &self,
        request: RunAgentTurnRequestDto,
        output: std::sync::Arc<dyn Fn(AgentTurnEventDto) + Send + Sync>,
    ) -> CommandResult<AgentTurnResultDto> {
        Self::command_result(crate::agent_turn::run_agent_turn(self, request, output).await)
    }

    /// Resolves the currently pending Agent tool approval.
    ///
    /// # Errors
    /// Returns an error envelope when there is no matching pending approval.
    pub fn decide_agent_approval(
        &self,
        request: DecideAgentApprovalRequestDto,
    ) -> CommandResult<()> {
        let session = self.current_session()?;
        let DecideAgentApprovalRequestDto {
            approval_id,
            approved,
        } = request;
        Self::command_result(
            session
                .agent_turn
                .decide(&approval_id, approved)
                .map_err(crate::AppError::from),
        )
    }

    /// Cancels the current workspace Agent turn without cancelling submitted generation work.
    ///
    /// # Errors
    /// Returns an error envelope when Agent runtime state is unavailable.
    pub fn cancel_agent_turn(&self, request: CancelAgentTurnRequestDto) -> CommandResult<bool> {
        let session = self.current_session()?;
        let session_id = request.session_id;
        Self::command_result(
            session
                .agent_turn
                .cancel(Some(&session_id))
                .map_err(crate::AppError::from),
        )
    }

    /// Reverts a tool mutation when the generation draft has not changed since it was applied.
    ///
    /// # Errors
    /// Returns an error envelope when the action is stale, missing, or persistence fails.
    pub async fn undo_agent_action(
        &self,
        request: UndoAgentActionRequestDto,
    ) -> CommandResult<VersionedGenerationDraftDto> {
        Self::command_result(crate::agent_turn::undo_agent_action(self, request).await)
    }
}
