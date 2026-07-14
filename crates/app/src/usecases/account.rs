use super::{
    ApiKeyId, ApiKeyRecordDto, AppError, AppResult, AtelierApp, CreateApiKeyRequestDto,
    NovelAiClientFactory, SecretStore, SecretValue, SubscriptionSummaryDto, UpdateApiKeyRequestDto,
    api_key_record_to_dto, create_api_key_to_domain, subscription_to_dto,
};

pub struct AccountUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> AccountUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequestDto,
    ) -> AppResult<ApiKeyRecordDto> {
        self.clear_active_subscription();
        self.app
            .inner
            .api_keys
            .create_api_key(create_api_key_to_domain(request))
            .await
            .map(|record| api_key_record_to_dto(&record))
            .map_err(AppError::from)
    }

    pub async fn update_api_key(
        &self,
        request: UpdateApiKeyRequestDto,
    ) -> AppResult<ApiKeyRecordDto> {
        self.clear_active_subscription();
        self.app
            .inner
            .api_keys
            .update_api_key(atelier_secrets::UpdateApiKeyRequest {
                id: ApiKeyId::new(request.id),
                display_name: request.display_name,
                secret: request.secret.map(SecretValue::new),
            })
            .await
            .map(|record| api_key_record_to_dto(&record))
            .map_err(AppError::from)
    }

    pub async fn delete_api_key(&self, id: &str) -> AppResult<bool> {
        self.clear_active_subscription();
        self.app
            .inner
            .api_keys
            .delete_api_key(&ApiKeyId::new(id))
            .await
            .map_err(AppError::from)
    }

    pub async fn list_api_keys(&self) -> AppResult<Vec<ApiKeyRecordDto>> {
        self.app
            .inner
            .api_keys
            .list_api_keys()
            .await
            .map(|items| items.iter().map(api_key_record_to_dto).collect())
            .map_err(AppError::from)
    }

    pub async fn set_active_api_key(&self, id: &str) -> AppResult<()> {
        self.clear_active_subscription();
        self.app
            .inner
            .api_keys
            .set_active_api_key(&ApiKeyId::new(id))
            .await
            .map_err(AppError::from)
    }

    pub async fn probe_key(&self, id: &str) -> AppResult<SubscriptionSummaryDto> {
        let summary = self
            .app
            .inner
            .api_keys
            .probe_key(&ApiKeyId::new(id))
            .await
            .map(|summary| subscription_to_dto(&summary))
            .map_err(AppError::from)?;
        let is_active = self
            .app
            .inner
            .api_keys
            .list_api_keys()
            .await?
            .into_iter()
            .any(|record| record.is_active && record.id.as_str() == id);
        if is_active {
            *self
                .app
                .inner
                .active_subscription
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(summary.clone());
        }
        Ok(summary)
    }

    pub async fn probe_active(&self) -> AppResult<SubscriptionSummaryDto> {
        let active = self
            .app
            .inner
            .api_keys
            .list_api_keys()
            .await?
            .into_iter()
            .find(|record| record.is_active)
            .ok_or_else(AppError::missing_active_key)?;
        self.probe_key(active.id.as_str()).await
    }

    #[must_use]
    pub fn cached_active_subscription(&self) -> Option<SubscriptionSummaryDto> {
        self.app
            .inner
            .active_subscription
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn clear_active_subscription(&self) {
        *self
            .app
            .inner
            .active_subscription
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }
}
