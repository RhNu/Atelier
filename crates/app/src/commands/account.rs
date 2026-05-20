use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
    ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, SubscriptionSummaryDto,
    UpdateApiKeyRequestDto,
};
use nai_atelier_secrets::SecretStore;

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    /// Creates API key metadata and stores the secret through the configured secret store.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or account storage fails.
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequestDto,
    ) -> CommandResult<ApiKeyRecordDto> {
        Self::command_result(self.current_app()?.account().create_api_key(request).await)
    }

    /// Updates API key metadata and optionally replaces its secret.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or account storage fails.
    pub async fn update_api_key(
        &self,
        request: UpdateApiKeyRequestDto,
    ) -> CommandResult<ApiKeyRecordDto> {
        Self::command_result(self.current_app()?.account().update_api_key(request).await)
    }

    /// Deletes an API key record and its stored secret.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or account storage fails.
    pub async fn delete_api_key(
        &self,
        request: DeleteApiKeyRequestDto,
    ) -> CommandResult<DeleteApiKeyResponseDto> {
        Self::command_result(
            self.current_app()?
                .account()
                .delete_api_key(&request.id)
                .await
                .map(|deleted| DeleteApiKeyResponseDto { deleted }),
        )
    }

    /// Lists configured API key records without secret values.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or account storage fails.
    pub async fn list_api_keys(&self) -> CommandResult<Vec<ApiKeyRecordDto>> {
        Self::command_result(self.current_app()?.account().list_api_keys().await)
    }

    /// Marks one API key as the active `NovelAI` key.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or account storage fails.
    pub async fn set_active_api_key(
        &self,
        request: SetActiveApiKeyRequestDto,
    ) -> CommandResult<()> {
        Self::command_result(
            self.current_app()?
                .account()
                .set_active_api_key(&request.id)
                .await,
        )
    }

    /// Probes one API key against `NovelAI` subscription status.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the key is missing, or `NovelAI` probe fails.
    pub async fn probe_api_key(
        &self,
        request: ProbeApiKeyRequestDto,
    ) -> CommandResult<SubscriptionSummaryDto> {
        Self::command_result(self.current_app()?.account().probe_key(&request.id).await)
    }

    /// Probes the active `NovelAI` API key.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, no key is active, or `NovelAI` probe fails.
    pub async fn probe_active_api_key(&self) -> CommandResult<SubscriptionSummaryDto> {
        Self::command_result(self.current_app()?.account().probe_active().await)
    }
}
