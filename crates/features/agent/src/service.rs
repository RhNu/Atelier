use std::sync::Arc;

use crate::{
    AgentAction, AgentActionId, AgentConnection, AgentConnectionId, AgentError, AgentEvent,
    AgentModel, AgentModelId, AgentModelSnapshot, AgentPersonaSnapshot, AgentRegistry,
    AgentRegistryRepository, AgentResult, AgentSession, AgentSessionId, AgentSessionStatus,
    AgentSummary, AgentWorkspaceRepository, AgentWorkspaceSettings,
};

#[derive(Clone)]
pub struct AgentRegistryService {
    repository: Arc<dyn AgentRegistryRepository>,
}

impl AgentRegistryService {
    #[must_use]
    pub fn new(repository: Arc<dyn AgentRegistryRepository>) -> Self {
        Self { repository }
    }

    /// Loads all application-global Agent connections and models.
    ///
    /// # Errors
    /// Returns an error when the registry cannot be read.
    pub async fn get_registry(&self) -> AgentResult<AgentRegistry> {
        self.repository.load_registry().await
    }

    /// Validates and creates or replaces a connection.
    ///
    /// # Errors
    /// Returns an error when validation or persistence fails.
    pub async fn save_connection(&self, connection: AgentConnection) -> AgentResult<AgentRegistry> {
        connection.validate()?;
        let mut registry = self.repository.load_registry().await?;
        upsert_by(&mut registry.connections, connection, |value| &value.id);
        registry.validate()?;
        self.repository.save_registry(registry.clone()).await?;
        Ok(registry)
    }

    /// Deletes an unused connection.
    ///
    /// # Errors
    /// Returns an error when it is missing, still referenced, or persistence fails.
    pub async fn delete_connection(&self, id: &AgentConnectionId) -> AgentResult<AgentRegistry> {
        let mut registry = self.repository.load_registry().await?;
        if registry
            .models
            .iter()
            .any(|model| &model.connection_id == id)
        {
            return Err(AgentError::conflict(
                "delete models that reference the connection first",
            ));
        }
        let before = registry.connections.len();
        registry
            .connections
            .retain(|connection| &connection.id != id);
        if before == registry.connections.len() {
            return Err(AgentError::not_found("agent connection does not exist"));
        }
        self.repository.save_registry(registry.clone()).await?;
        Ok(registry)
    }

    /// Validates and creates or replaces a model.
    ///
    /// # Errors
    /// Returns an error when validation, connection lookup, or persistence fails.
    pub async fn save_model(&self, model: AgentModel) -> AgentResult<AgentRegistry> {
        model.validate()?;
        let mut registry = self.repository.load_registry().await?;
        upsert_by(&mut registry.models, model, |value| &value.id);
        registry.validate()?;
        self.repository.save_registry(registry.clone()).await?;
        Ok(registry)
    }

    /// Deletes a model definition.
    ///
    /// # Errors
    /// Returns an error when it is missing or persistence fails.
    pub async fn delete_model(&self, id: &AgentModelId) -> AgentResult<AgentRegistry> {
        let mut registry = self.repository.load_registry().await?;
        let before = registry.models.len();
        registry.models.retain(|model| &model.id != id);
        if before == registry.models.len() {
            return Err(AgentError::not_found("agent model does not exist"));
        }
        self.repository.save_registry(registry.clone()).await?;
        Ok(registry)
    }
}

fn upsert_by<T, K: PartialEq>(values: &mut Vec<T>, value: T, key: impl Fn(&T) -> &K) {
    if let Some(current) = values
        .iter_mut()
        .find(|current| key(current) == key(&value))
    {
        *current = value;
    } else {
        values.push(value);
    }
}

#[derive(Clone)]
pub struct AgentWorkspaceService<R> {
    repository: R,
}

impl<R> AgentWorkspaceService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R: AgentWorkspaceRepository> AgentWorkspaceService<R> {
    /// Loads workspace Agent personalization and permission settings.
    ///
    /// # Errors
    /// Returns an error when the workspace repository cannot be read.
    pub async fn get_settings(&self) -> AgentResult<AgentWorkspaceSettings> {
        self.repository.get_settings().await
    }

    /// Validates and replaces workspace Agent settings.
    ///
    /// # Errors
    /// Returns an error when validation or persistence fails.
    pub async fn update_settings(
        &self,
        settings: AgentWorkspaceSettings,
    ) -> AgentResult<AgentWorkspaceSettings> {
        settings.validate()?;
        self.repository.save_settings(settings.clone()).await?;
        Ok(settings)
    }

    /// Creates a durable session using current model and persona snapshots.
    ///
    /// # Errors
    /// Returns an error for an invalid title, missing model, or persistence failure.
    pub async fn create_session(
        &self,
        id: AgentSessionId,
        title: String,
        model_id: &AgentModelId,
        registry: &AgentRegistry,
        now_ms: u64,
    ) -> AgentResult<AgentSession> {
        if title.trim().is_empty() || title.chars().count() > 80 {
            return Err(AgentError::validation(
                "agent session title must contain between 1 and 80 characters",
            ));
        }
        let settings = self.repository.get_settings().await?;
        let model = registry
            .models
            .iter()
            .find(|model| &model.id == model_id)
            .ok_or_else(|| AgentError::not_found("agent model does not exist"))?;
        let session = AgentSession {
            id,
            title,
            model: AgentModelSnapshot {
                model_id: model.id.clone(),
                connection_id: model.connection_id.clone(),
                wire_model_id: model.wire_model_id.clone(),
                display_name: model.display_name.clone(),
                context_window: model.context_window,
                max_output_tokens: model.max_output_tokens,
                temperature: model.temperature,
                supports_vision: model.supports_vision,
            },
            persona: AgentPersonaSnapshot {
                display_name: settings.display_name,
                instructions: settings.instructions,
                v5_prompt_guidance: settings.v5_prompt_guidance,
                tag_prompt_guidance: settings.tag_prompt_guidance,
            },
            status: AgentSessionStatus::Idle,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };
        self.repository.save_session(session.clone()).await?;
        Ok(session)
    }

    /// Lists durable sessions in repository order.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read.
    pub async fn list_sessions(&self) -> AgentResult<Vec<AgentSession>> {
        self.repository.list_sessions().await
    }

    /// Loads one durable session.
    ///
    /// # Errors
    /// Returns an error when the session is missing or the repository cannot be read.
    pub async fn get_session(&self, id: &AgentSessionId) -> AgentResult<AgentSession> {
        self.repository
            .get_session(id)
            .await?
            .ok_or_else(|| AgentError::not_found("agent session does not exist"))
    }

    /// Creates or replaces one session.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn save_session(&self, session: AgentSession) -> AgentResult<()> {
        if session.title.trim().is_empty() || session.title.chars().count() > 80 {
            return Err(AgentError::validation(
                "agent session title must contain between 1 and 80 characters",
            ));
        }
        self.repository.save_session(session).await
    }

    /// Renames an idle or interrupted session.
    ///
    /// # Errors
    /// Returns an error for an invalid title, missing session, or persistence failure.
    pub async fn rename_session(
        &self,
        id: &AgentSessionId,
        title: String,
        now_ms: u64,
    ) -> AgentResult<AgentSession> {
        let mut session = self.get_session(id).await?;
        if session.status == AgentSessionStatus::Running {
            return Err(AgentError::conflict(
                "stop the active Agent turn before renaming",
            ));
        }
        session.title = title;
        session.updated_at_ms = now_ms;
        self.save_session(session.clone()).await?;
        Ok(session)
    }

    /// Deletes a session and its owned records.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn delete_session(&self, id: &AgentSessionId) -> AgentResult<bool> {
        if let Some(session) = self.repository.get_session(id).await?
            && session.status == AgentSessionStatus::Running
        {
            return Err(AgentError::conflict(
                "stop the active Agent turn before deleting its session",
            ));
        }
        self.repository.delete_session(id).await
    }

    /// Lists a session's complete event log.
    ///
    /// # Errors
    /// Returns an error when the repository cannot be read.
    pub async fn list_events(&self, id: &AgentSessionId) -> AgentResult<Vec<AgentEvent>> {
        self.repository.list_events(id).await
    }

    /// Appends a durable session event.
    ///
    /// # Errors
    /// Returns an error for duplicate sequence values or persistence failure.
    pub async fn append_event(&self, event: AgentEvent) -> AgentResult<()> {
        self.repository.append_event(event).await
    }

    /// Loads one reversible Agent action.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn get_action(&self, id: &AgentActionId) -> AgentResult<Option<AgentAction>> {
        self.repository.get_action(id).await
    }

    /// Creates or updates one reversible Agent action.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn save_action(&self, action: AgentAction) -> AgentResult<()> {
        self.repository.save_action(action).await
    }

    /// Loads the latest rolling summary for a session.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn get_summary(&self, id: &AgentSessionId) -> AgentResult<Option<AgentSummary>> {
        self.repository.get_summary(id).await
    }

    /// Replaces the rolling summary for a session.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn save_summary(&self, summary: AgentSummary) -> AgentResult<()> {
        self.repository.save_summary(summary).await
    }

    /// Marks unfinished sessions as interrupted during workspace startup.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn interrupt_running_sessions(&self, now_ms: u64) -> AgentResult<usize> {
        self.repository.interrupt_running_sessions(now_ms).await
    }
}
