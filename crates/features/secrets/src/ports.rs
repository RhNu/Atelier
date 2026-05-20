use async_trait::async_trait;
use nai_atelier_foundation::NovelAiError;

use crate::{
    ApiKeyId, ApiKeyRecord, SecretRecordId, SecretValue, SecretsResult, SubscriptionSummary,
};

pub type SubscriptionResult<T> = Result<T, NovelAiError>;

#[async_trait]
pub trait ApiKeyRegistryStore: Send + Sync {
    async fn insert_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()>;

    async fn save_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()>;

    async fn get_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<Option<ApiKeyRecord>>;

    async fn list_api_key_records(&self) -> SecretsResult<Vec<ApiKeyRecord>>;

    async fn delete_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<bool>;

    async fn set_active_api_key(&self, id: &ApiKeyId) -> SecretsResult<()>;

    async fn get_active_api_key_record(&self) -> SecretsResult<Option<ApiKeyRecord>>;
}

#[async_trait]
pub trait SecretStore: Send + Sync {
    async fn write_secret(&self, id: &SecretRecordId, secret: SecretValue) -> SecretsResult<()>;

    async fn read_secret(&self, id: &SecretRecordId) -> SecretsResult<SecretValue>;

    async fn delete_secret(&self, id: &SecretRecordId) -> SecretsResult<bool>;
}

#[async_trait]
pub trait SecretResolver: Send + Sync {
    async fn resolve_active_secret(&self) -> SecretsResult<SecretValue>;
}

#[async_trait]
pub trait SubscriptionClient: Send + Sync {
    async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary>;
}

#[async_trait]
pub trait SubscriptionProbeClient: Send + Sync {
    async fn probe_subscription(
        &self,
        secret: SecretValue,
    ) -> SubscriptionResult<SubscriptionSummary>;
}
