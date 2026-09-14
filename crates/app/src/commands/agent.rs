use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::agent::{
    AgentRegistryDto, DeleteAgentConnectionRequestDto, DeleteAgentModelRequestDto,
    SaveAgentConnectionRequestDto, SaveAgentModelRequestDto,
};
use atelier_secrets::SecretStore;

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    /// Returns application-global Agent model configuration.
    ///
    /// # Errors
    /// Returns an error envelope when registry persistence fails.
    pub async fn get_agent_registry(&self) -> CommandResult<AgentRegistryDto> {
        Self::command_result(self.agent_models().get_registry().await)
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
}
