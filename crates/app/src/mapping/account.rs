use super::{
    ApiKeyId, ApiKeyRecord, ApiKeyRecordDto, CreateApiKeyRequest, SecretValue,
    SubscriptionSummaryDto,
};

pub fn api_key_record_to_dto(record: &ApiKeyRecord) -> ApiKeyRecordDto {
    ApiKeyRecordDto {
        id: record.id.as_str().to_owned(),
        display_name: record.display_name.clone(),
        is_active: record.is_active,
    }
}

pub fn create_api_key_to_domain(
    request: atelier_app_api::account::CreateApiKeyRequestDto,
) -> CreateApiKeyRequest {
    CreateApiKeyRequest {
        id: ApiKeyId::new(request.id),
        display_name: request.display_name,
        secret: SecretValue::new(request.secret),
    }
}

pub fn subscription_to_dto(value: &atelier_secrets::SubscriptionSummary) -> SubscriptionSummaryDto {
    SubscriptionSummaryDto {
        anlas_balance: value.anlas_balance,
        is_opus: value.is_opus,
        tier: value.tier,
        tier_name: value.tier_name.clone(),
        expires_at_ms: value.expires_at_ms,
    }
}
