use async_trait::async_trait;
use nai_atelier_foundation::NovelAiError;

pub type SecretsResult<T> = Result<T, NovelAiError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionSummary {
    pub anlas_balance: i64,
    pub is_opus: bool,
    pub tier: i32,
    pub tier_name: String,
    pub expires_at_ms: Option<u64>,
}

#[async_trait]
pub trait SubscriptionClient: Send + Sync {
    async fn get_subscription(&self) -> SecretsResult<SubscriptionSummary>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-secrets");
    }

    #[test]
    fn subscription_summary_keeps_novelai_account_fields() {
        let summary = SubscriptionSummary {
            anlas_balance: 42,
            is_opus: true,
            tier: 3,
            tier_name: "opus".to_owned(),
            expires_at_ms: Some(1_700_000_000_000),
        };

        assert_eq!(summary.tier_name, "opus");
        assert_eq!(summary.expires_at_ms, Some(1_700_000_000_000));
    }
}
