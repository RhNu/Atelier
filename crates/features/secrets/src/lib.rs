mod error;
mod model;
mod ports;
mod service;

pub use error::{
    ClientApiErrorContext, ClientApiErrorReason, ClientDecodeContext, ClientDecodeTarget,
    ClientInvalidRequestContext, ClientInvalidRequestKind, ClientMetadataContext,
    ClientMetadataKind, ClientTransportContext, ClientTransportOperation, ProbeApiKeyError,
    ProbeApiKeyResult, SecretsError, SecretsErrorKind, SecretsResult, SubscriptionClientError,
};
pub use model::{
    ApiKeyId, ApiKeyRecord, CreateApiKeyRequest, SecretRecordId, SecretValue, SubscriptionSummary,
    UpdateApiKeyRequest, V5UsageStatus,
};
pub use ports::{
    ApiKeyRegistryStore, SecretResolver, SecretStore, SubscriptionClient, SubscriptionProbeClient,
    SubscriptionResult,
};
pub use service::ApiKeyRegistryService;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "atelier-secrets");
    }

    #[test]
    fn subscription_summary_keeps_novelai_account_fields() {
        let summary = SubscriptionSummary {
            anlas_balance: 42,
            is_opus: true,
            subscription_active: true,
            tier: 3,
            tier_name: "opus".to_owned(),
            expires_at_ms: Some(1_700_000_000_000),
            v5_usage: None,
        };

        assert_eq!(summary.tier_name, "opus");
        assert_eq!(summary.expires_at_ms, Some(1_700_000_000_000));
    }
}
