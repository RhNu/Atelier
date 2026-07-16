use async_trait::async_trait;
use futures::lock::Mutex;
use std::sync::Arc;

use crate::{
    ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, CreateApiKeyRequest, ProbeApiKeyResult,
    SecretRecordId, SecretResolver, SecretStore, SecretValue, SecretsError, SecretsErrorKind,
    SecretsResult, SubscriptionProbeClient, SubscriptionSummary, UpdateApiKeyRequest,
};

#[derive(Clone, Debug)]
pub struct ApiKeyRegistryService<M, S, P> {
    metadata: M,
    secrets: S,
    probe: P,
    operation_gate: Arc<Mutex<()>>,
}

impl<M, S, P> ApiKeyRegistryService<M, S, P> {
    #[must_use]
    pub fn new(metadata: M, secrets: S, probe: P) -> Self {
        Self {
            metadata,
            secrets,
            probe,
            operation_gate: Arc::new(Mutex::new(())),
        }
    }
}

impl<M, S, P> ApiKeyRegistryService<M, S, P>
where
    M: ApiKeyRegistryStore,
    S: SecretStore,
    P: SubscriptionProbeClient,
{
    /// Creates API key metadata and stores the secret value.
    ///
    /// # Errors
    /// Returns an error when validation fails, the secret store fails, or
    /// metadata persistence fails.
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequest,
    ) -> SecretsResult<ApiKeyRecord> {
        let _gate = self.operation_gate.lock().await;
        validate_id(&request.id)?;
        let display_name = validate_display_name(&request.display_name)?;
        validate_secret(&request.secret)?;

        let record = ApiKeyRecord {
            secret_record_id: SecretRecordId::for_api_key(&request.id),
            id: request.id,
            display_name,
            is_active: false,
        };
        self.metadata.insert_api_key_record(record.clone()).await?;
        if let Err(error) = self
            .secrets
            .write_secret(&record.secret_record_id, request.secret)
            .await
        {
            let mut cleanup_failures = Vec::new();
            if let Err(cleanup_error) = self.secrets.delete_secret(&record.secret_record_id).await {
                cleanup_failures.push(format!("secret cleanup failed: {cleanup_error}"));
            }
            if let Err(cleanup_error) = self.metadata.delete_api_key_record(&record.id).await {
                cleanup_failures.push(format!("metadata cleanup failed: {cleanup_error}"));
            }
            if !cleanup_failures.is_empty() {
                return Err(SecretsError::secret_store(format!(
                    "secret write failed: {error}; {}",
                    cleanup_failures.join("; ")
                )));
            }
            return Err(error);
        }
        Ok(record)
    }

    /// Updates API key metadata and optionally replaces the stored secret.
    ///
    /// # Errors
    /// Returns an error when the key is missing, validation fails, the secret
    /// store fails, or metadata persistence fails.
    pub async fn update_api_key(
        &self,
        request: UpdateApiKeyRequest,
    ) -> SecretsResult<ApiKeyRecord> {
        let _gate = self.operation_gate.lock().await;
        validate_id(&request.id)?;
        let mut record = self
            .metadata
            .get_api_key_record(&request.id)
            .await?
            .ok_or_else(|| SecretsError::metadata_store("api key does not exist"))?;
        let previous_record = record.clone();
        if let Some(display_name) = request.display_name {
            record.display_name = validate_display_name(&display_name)?;
        }
        let replacement_secret = if let Some(secret) = request.secret {
            validate_secret(&secret)?;
            Some(secret)
        } else {
            None
        };
        let previous_secret = if replacement_secret.is_some() {
            match self.secrets.read_secret(&record.secret_record_id).await {
                Ok(secret) => Some(secret),
                Err(error) if error.kind == SecretsErrorKind::MissingSecret => None,
                Err(error) => return Err(error),
            }
        } else {
            None
        };
        let metadata_changed = record != previous_record;
        if metadata_changed {
            self.metadata.save_api_key_record(record.clone()).await?;
        }
        if let Some(secret) = replacement_secret
            && let Err(error) = self
                .secrets
                .write_secret(&record.secret_record_id, secret)
                .await
        {
            let mut rollback_failures = Vec::new();
            if let Some(previous_secret) = previous_secret {
                if let Err(rollback_error) = self
                    .secrets
                    .write_secret(&record.secret_record_id, previous_secret)
                    .await
                {
                    rollback_failures.push(format!("secret rollback failed: {rollback_error}"));
                }
            } else if let Err(rollback_error) =
                self.secrets.delete_secret(&record.secret_record_id).await
            {
                rollback_failures.push(format!("secret cleanup failed: {rollback_error}"));
            }
            if metadata_changed
                && let Err(rollback_error) =
                    self.metadata.save_api_key_record(previous_record).await
            {
                rollback_failures.push(format!("metadata rollback failed: {rollback_error}"));
            }
            if !rollback_failures.is_empty() {
                return Err(SecretsError::secret_store(format!(
                    "secret write failed: {error}; {}",
                    rollback_failures.join("; ")
                )));
            }
            return Err(error);
        }
        Ok(record)
    }

    /// Deletes API key metadata and its paired secret.
    ///
    /// # Errors
    /// Returns an error when validation, metadata deletion, or secret deletion
    /// fails.
    pub async fn delete_api_key(&self, id: &ApiKeyId) -> SecretsResult<bool> {
        let _gate = self.operation_gate.lock().await;
        validate_id(id)?;
        let Some(record) = self.metadata.get_api_key_record(id).await? else {
            return Ok(false);
        };
        let previous_secret = match self.secrets.read_secret(&record.secret_record_id).await {
            Ok(secret) => Some(secret),
            Err(error) if error.kind == SecretsErrorKind::MissingSecret => None,
            Err(error) => return Err(error),
        };
        let _deleted_secret = self.secrets.delete_secret(&record.secret_record_id).await?;
        let deleted = match self.metadata.delete_api_key_record(id).await {
            Ok(deleted) => deleted,
            Err(error) => {
                if let Some(secret) = previous_secret {
                    self.secrets
                        .write_secret(&record.secret_record_id, secret)
                        .await
                        .map_err(|restore_error| {
                            SecretsError::secret_store(format!(
                                "metadata delete failed: {error}; secret restore failed: {restore_error}"
                            ))
                        })?;
                }
                return Err(error);
            }
        };
        Ok(deleted)
    }

    /// Lists all API key metadata records.
    ///
    /// # Errors
    /// Returns an error when metadata retrieval fails.
    pub async fn list_api_keys(&self) -> SecretsResult<Vec<ApiKeyRecord>> {
        self.metadata.list_api_key_records().await
    }

    /// Marks one API key as active.
    ///
    /// # Errors
    /// Returns an error when validation fails or the metadata store cannot mark
    /// the key active.
    pub async fn set_active_api_key(&self, id: &ApiKeyId) -> SecretsResult<()> {
        let _gate = self.operation_gate.lock().await;
        validate_id(id)?;
        self.metadata.set_active_api_key(id).await
    }

    /// Resolves the secret for a specific API key id.
    ///
    /// # Errors
    /// Returns an error when validation fails, metadata lookup fails, the key is
    /// missing, or the secret is missing.
    pub async fn resolve_secret_for_key(&self, id: &ApiKeyId) -> SecretsResult<SecretValue> {
        let _gate = self.operation_gate.lock().await;
        validate_id(id)?;
        let record = self
            .metadata
            .get_api_key_record(id)
            .await?
            .ok_or_else(|| SecretsError::metadata_store("api key does not exist"))?;
        self.secrets.read_secret(&record.secret_record_id).await
    }

    /// Resolves the currently active API key secret.
    ///
    /// # Errors
    /// Returns an error when there is no active key or its secret cannot be
    /// resolved.
    pub async fn resolve_active_secret(&self) -> SecretsResult<SecretValue> {
        let _gate = self.operation_gate.lock().await;
        let record = self
            .metadata
            .get_active_api_key_record()
            .await?
            .ok_or_else(SecretsError::missing_active_key)?;
        self.secrets.read_secret(&record.secret_record_id).await
    }

    /// Probes `NovelAI` subscription information for a specific API key.
    ///
    /// # Errors
    /// Returns a secret resolution error when the key cannot be resolved, or a
    /// `NovelAI` error when the probe request fails.
    pub async fn probe_key(&self, id: &ApiKeyId) -> ProbeApiKeyResult<SubscriptionSummary> {
        let secret = self.resolve_secret_for_key(id).await?;
        self.probe
            .probe_subscription(secret)
            .await
            .map_err(Into::into)
    }
}

#[async_trait]
impl<M, S, P> SecretResolver for ApiKeyRegistryService<M, S, P>
where
    M: ApiKeyRegistryStore,
    S: SecretStore,
    P: Send + Sync,
{
    async fn resolve_active_secret(&self) -> SecretsResult<SecretValue> {
        let _gate = self.operation_gate.lock().await;
        let record = self
            .metadata
            .get_active_api_key_record()
            .await?
            .ok_or_else(SecretsError::missing_active_key)?;
        self.secrets.read_secret(&record.secret_record_id).await
    }
}

fn validate_id(id: &ApiKeyId) -> SecretsResult<()> {
    if id.as_str().trim().is_empty() {
        Err(SecretsError::validation("api key id must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_display_name(value: &str) -> SecretsResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(SecretsError::validation("display name must not be empty"))
    } else {
        Ok(trimmed.to_owned())
    }
}

fn validate_secret(secret: &SecretValue) -> SecretsResult<()> {
    if secret.expose_secret().trim().is_empty() {
        Err(SecretsError::validation("secret must not be empty"))
    } else {
        Ok(())
    }
}
