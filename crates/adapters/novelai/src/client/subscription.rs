use super::{
    NovelAiClientFactory, ReqwestNovelAiClientFactory, SecretValue, SubscriptionClient,
    SubscriptionProbeClient, SubscriptionResult, SubscriptionSummary, async_trait,
    map_subscription_error,
};

#[derive(Clone, Debug)]
pub struct NovelAiSubscriptionProbeClient<F = ReqwestNovelAiClientFactory> {
    factory: F,
}

impl<F> NovelAiSubscriptionProbeClient<F> {
    #[must_use]
    pub const fn new(factory: F) -> Self {
        Self { factory }
    }
}

#[async_trait]
impl<F> SubscriptionProbeClient for NovelAiSubscriptionProbeClient<F>
where
    F: NovelAiClientFactory,
{
    async fn probe_subscription(
        &self,
        secret: SecretValue,
    ) -> SubscriptionResult<SubscriptionSummary> {
        self.factory
            .create_client(secret)
            .map_err(map_subscription_error)?
            .get_subscription()
            .await
    }
}
