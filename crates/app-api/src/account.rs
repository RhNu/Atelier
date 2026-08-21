use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CreateApiKeyRequestDto {
    pub id: String,
    pub display_name: String,
    pub secret: String,
}

impl std::fmt::Debug for CreateApiKeyRequestDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateApiKeyRequestDto")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("secret", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct UpdateApiKeyRequestDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

impl std::fmt::Debug for UpdateApiKeyRequestDto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UpdateApiKeyRequestDto")
            .field("id", &self.id)
            .field("display_name", &self.display_name)
            .field("secret", &self.secret.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteApiKeyRequestDto {
    pub id: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteApiKeyResponseDto {
    pub deleted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SetActiveApiKeyRequestDto {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProbeApiKeyRequestDto {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ApiKeyRecordDto {
    pub id: String,
    pub display_name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SubscriptionSummaryDto {
    pub anlas_balance: i64,
    pub is_opus: bool,
    pub tier: i32,
    pub tier_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v5_usage: Option<V5UsageStatusDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct V5UsageStatusDto {
    pub is_negative: bool,
    pub percent: u32,
    pub seconds_until_next_percent: u64,
}
