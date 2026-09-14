use atelier_agent::{
    AgentAuth, AgentConnectionId, AgentModelId, AgentModelRuntime, AgentRegistryService,
    AgentResolvedConnection,
};
use atelier_app_api::agent::{
    AgentRegistryDto, DiscoveredAgentModelDto, SaveAgentConnectionRequestDto,
    SaveAgentModelRequestDto,
};
use atelier_secrets::{SecretRecordId, SecretStore, SecretValue};

use crate::mapping::{agent_connection_to_domain, agent_model_to_domain, agent_registry_to_dto};
use crate::{AppError, AppResult};

pub struct AgentModelUseCases<'a, S> {
    pub(crate) registry: &'a AgentRegistryService,
    pub(crate) secrets: &'a S,
    pub(crate) runtime: &'a std::sync::Arc<dyn AgentModelRuntime>,
}

impl<S> AgentModelUseCases<'_, S>
where
    S: SecretStore + Send + Sync,
{
    /// Returns global Agent connection and model metadata without secret values.
    ///
    /// # Errors
    /// Returns an error when registry persistence fails.
    pub async fn get_registry(&self) -> AppResult<AgentRegistryDto> {
        self.registry
            .get_registry()
            .await
            .map(|registry| agent_registry_to_dto(&registry))
            .map_err(AppError::from)
    }

    /// Lists models exposed by one configured OpenAI-compatible connection.
    ///
    /// # Errors
    /// Returns an error when the connection, secret, or provider request is unavailable.
    pub async fn discover_models(
        &self,
        connection_id: &str,
    ) -> AppResult<Vec<DiscoveredAgentModelDto>> {
        let registry = self.registry.get_registry().await?;
        let connection = registry
            .connections
            .iter()
            .find(|value| value.id.as_str() == connection_id)
            .ok_or_else(|| AppError::new("agent_not_found", "agent connection does not exist"))?;
        let bearer_token = match &connection.auth {
            AgentAuth::None => None,
            AgentAuth::Bearer { secret_record_id } => Some(
                self.secrets
                    .read_secret(&SecretRecordId::new(secret_record_id.clone()))
                    .await?
                    .expose_secret()
                    .to_owned(),
            ),
        };
        self.runtime
            .discover_models(AgentResolvedConnection {
                base_url: connection.base_url.clone(),
                bearer_token,
            })
            .await
            .map(|models| {
                models
                    .into_iter()
                    .map(|model| DiscoveredAgentModelDto {
                        wire_model_id: model.wire_model_id,
                        display_name: model.display_name,
                        context_window: model.context_window,
                        max_output_tokens: model.max_output_tokens,
                    })
                    .collect()
            })
            .map_err(AppError::from)
    }

    /// Creates or updates a connection and its optional keyring secret.
    ///
    /// # Errors
    /// Returns an error when validation, registry persistence, or secret storage fails.
    pub async fn save_connection(
        &self,
        request: SaveAgentConnectionRequestDto,
    ) -> AppResult<AgentRegistryDto> {
        let registry = self.registry.get_registry().await?;
        let current = registry
            .connections
            .iter()
            .find(|value| value.id.as_str() == request.id);
        let current_secret = current.and_then(secret_record_id);
        let requested_secret = request.secret.clone();
        if matches!(
            request.auth_kind,
            atelier_app_api::agent::AgentAuthKindDto::Bearer
        ) && requested_secret.is_none()
            && current_secret.is_none()
        {
            return Err(AppError::new(
                "agent_validation",
                "a bearer secret is required when creating the connection",
            ));
        }
        let connection =
            agent_connection_to_domain(request, current, crate::time::unix_timestamp_ms());
        connection.validate()?;

        if let (AgentAuth::Bearer { secret_record_id }, Some(secret)) =
            (&connection.auth, requested_secret)
        {
            self.secrets
                .write_secret(
                    &SecretRecordId::new(secret_record_id.clone()),
                    SecretValue::new(secret),
                )
                .await?;
        }
        let registry = self.registry.save_connection(connection.clone()).await?;
        if matches!(connection.auth, AgentAuth::None)
            && let Some(secret_record_id) = current_secret
        {
            self.secrets
                .delete_secret(&SecretRecordId::new(secret_record_id))
                .await?;
        }
        Ok(agent_registry_to_dto(&registry))
    }

    /// Deletes an unused connection and its keyring secret.
    ///
    /// # Errors
    /// Returns an error when the connection is missing, referenced, or persistence fails.
    pub async fn delete_connection(&self, id: &str) -> AppResult<AgentRegistryDto> {
        let registry = self.registry.get_registry().await?;
        let connection = registry
            .connections
            .iter()
            .find(|value| value.id.as_str() == id)
            .ok_or_else(|| AppError::new("agent_not_found", "agent connection does not exist"))?;
        let secret = secret_record_id(connection);
        let registry = self
            .registry
            .delete_connection(&AgentConnectionId::new(id))
            .await?;
        if let Some(secret) = secret {
            self.secrets
                .delete_secret(&SecretRecordId::new(secret))
                .await?;
        }
        Ok(agent_registry_to_dto(&registry))
    }

    /// Creates or updates a model definition.
    ///
    /// # Errors
    /// Returns an error when validation or persistence fails.
    pub async fn save_model(
        &self,
        request: SaveAgentModelRequestDto,
    ) -> AppResult<AgentRegistryDto> {
        let model = agent_model_to_domain(request, crate::time::unix_timestamp_ms());
        self.registry
            .save_model(model)
            .await
            .map(|registry| agent_registry_to_dto(&registry))
            .map_err(AppError::from)
    }

    /// Deletes a model definition.
    ///
    /// # Errors
    /// Returns an error when the model is missing or persistence fails.
    pub async fn delete_model(&self, id: &str) -> AppResult<AgentRegistryDto> {
        self.registry
            .delete_model(&AgentModelId::new(id))
            .await
            .map(|registry| agent_registry_to_dto(&registry))
            .map_err(AppError::from)
    }
}

fn secret_record_id(connection: &atelier_agent::AgentConnection) -> Option<String> {
    match &connection.auth {
        AgentAuth::None => None,
        AgentAuth::Bearer { secret_record_id } => Some(secret_record_id.clone()),
    }
}
