#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiKeyId(String);

impl ApiKeyId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecretRecordId(String);

impl SecretRecordId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn for_api_key(id: &ApiKeyId) -> Self {
        Self(format!("novelai-api-key:{}", id.as_str()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue(String);

impl SecretValue {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SecretValue").field(&"<redacted>").finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiKeyRecord {
    pub id: ApiKeyId,
    pub display_name: String,
    pub secret_record_id: SecretRecordId,
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateApiKeyRequest {
    pub id: ApiKeyId,
    pub display_name: String,
    pub secret: SecretValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateApiKeyRequest {
    pub id: ApiKeyId,
    pub display_name: Option<String>,
    pub secret: Option<SecretValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionSummary {
    pub anlas_balance: i64,
    pub is_opus: bool,
    pub tier: i32,
    pub tier_name: String,
    pub expires_at_ms: Option<u64>,
    pub v5_usage: Option<V5UsageStatus>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct V5UsageStatus {
    pub is_negative: bool,
    pub percent: u32,
    pub seconds_until_next_percent: u64,
}
