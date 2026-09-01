use super::{
    BridgeApiErrorContext, BridgeApiErrorReason, BridgeDecodeContext, BridgeDecodeTarget,
    BridgeInvalidRequestContext, BridgeInvalidRequestKind, BridgeMetadataContext,
    BridgeMetadataKind, BridgeTransportContext, BridgeTransportOperation, NovelAiBridgeError,
};
use novelai_bridge as bridge;

pub fn map_bridge_error(error: bridge::BridgeError) -> NovelAiBridgeError {
    match error {
        bridge::BridgeError::InvalidRequest(error) => {
            let message = error.to_string();
            let context = map_invalid_request_context(error);
            NovelAiBridgeError::invalid_request_with_contexts(None, context, None, message)
        }
        bridge::BridgeError::Api(error) => {
            let message = error.to_string();
            let status = error.status;
            let context = Some(BridgeApiErrorContext {
                endpoint: error.endpoint,
                server_reason: error.server_reason.map(map_api_reason),
                raw_body: error.raw_body,
            });
            match error.kind {
                bridge::ApiErrorKind::InvalidRequest => {
                    NovelAiBridgeError::invalid_request_with_contexts(
                        Some(status),
                        None,
                        context,
                        message,
                    )
                }
                bridge::ApiErrorKind::AuthenticationFailed => {
                    NovelAiBridgeError::authentication_with_context(Some(status), context, message)
                }
                bridge::ApiErrorKind::InsufficientCredit => {
                    NovelAiBridgeError::insufficient_credit_with_context(
                        Some(status),
                        context,
                        message,
                    )
                }
                bridge::ApiErrorKind::RequestConflict => {
                    NovelAiBridgeError::request_conflict_with_context(
                        Some(status),
                        context,
                        message,
                    )
                }
                bridge::ApiErrorKind::UnexpectedStatus => {
                    NovelAiBridgeError::unknown_api_with_context(Some(status), context, message)
                }
                bridge::ApiErrorKind::RateLimited { retry_after } => {
                    NovelAiBridgeError::rate_limited_with_context(
                        status,
                        retry_after,
                        context,
                        message,
                    )
                }
                bridge::ApiErrorKind::ServerError => {
                    NovelAiBridgeError::service_unavailable_with_context(
                        Some(status),
                        context,
                        message,
                    )
                }
                _ => NovelAiBridgeError::unknown_api_with_context(Some(status), context, message),
            }
        }
        bridge::BridgeError::Transport(error) => {
            let message = error.to_string();
            let context = BridgeTransportContext {
                operation: map_transport_operation(error.operation),
                endpoint: error.endpoint,
                source: error.source.to_string(),
            };
            NovelAiBridgeError::transport_with_context(Some(context), message)
        }
        bridge::BridgeError::Decode(error) => {
            let message = error.to_string();
            let context = BridgeDecodeContext {
                target: map_decode_target(error.target),
                source: error.source.to_string(),
            };
            NovelAiBridgeError::decode_with_context(Some(context), message)
        }
        bridge::BridgeError::Metadata(error) => {
            let message = error.to_string();
            let context = map_metadata_context(error);
            NovelAiBridgeError::metadata_with_context(context, message)
        }
        other => NovelAiBridgeError::unknown_api(None, other.to_string()),
    }
}

fn map_invalid_request_context(
    error: bridge::InvalidRequest,
) -> Option<BridgeInvalidRequestContext> {
    use BridgeInvalidRequestKind as Kind;
    use bridge::InvalidRequest as Source;

    let context = match error {
        Source::PromptTokenLimitExceeded { field, used, limit } => BridgeInvalidRequestContext {
            field: Some(field),
            value: Some(used.to_string()),
            max: Some(limit.to_string()),
            ..invalid_request_context(Kind::NumericOutOfRange)
        },
        Source::EmptyField { field } => field_context(Kind::EmptyField, field),
        Source::MissingConfiguration { name } => BridgeInvalidRequestContext {
            name: Some(name),
            ..invalid_request_context(Kind::MissingConfiguration)
        },
        Source::NumericOutOfRange {
            field,
            value,
            min,
            max,
        } => BridgeInvalidRequestContext {
            field: Some(field),
            value: Some(value.to_string()),
            min: Some(min.to_string()),
            max: Some(max.to_string()),
            ..invalid_request_context(Kind::NumericOutOfRange)
        },
        Source::InvalidImageDimension {
            field,
            value,
            min,
            max,
            multiple_of,
        } => BridgeInvalidRequestContext {
            field: Some(field),
            value: Some(value.to_string()),
            min: Some(min.to_string()),
            max: Some(max.to_string()),
            multiple_of: Some(multiple_of),
            ..invalid_request_context(Kind::InvalidImageDimension)
        },
        Source::NonFiniteNumber { field, value } => BridgeInvalidRequestContext {
            field: Some(field),
            value: Some(value.to_string()),
            ..invalid_request_context(Kind::NonFiniteNumber)
        },
        Source::InvalidDataUrl { field, reason } => BridgeInvalidRequestContext {
            field: Some(field),
            reason: Some(reason.to_string()),
            ..invalid_request_context(Kind::InvalidDataUrl)
        },
        Source::InvalidBase64 { field, source } => BridgeInvalidRequestContext {
            field: Some(field),
            source: Some(source.to_string()),
            ..invalid_request_context(Kind::InvalidBase64)
        },
        Source::UndecodableImage { field, source } => BridgeInvalidRequestContext {
            field: Some(field),
            source: Some(source.to_string()),
            ..invalid_request_context(Kind::UndecodableImage)
        },
        Source::UnsupportedModelFeature {
            feature,
            required_model,
        } => BridgeInvalidRequestContext {
            feature: Some(feature),
            required_model: Some(required_model),
            ..invalid_request_context(Kind::UnsupportedModelFeature)
        },
        Source::UnsupportedFieldCombination { left, right } => BridgeInvalidRequestContext {
            left: Some(left),
            right: Some(right),
            ..invalid_request_context(Kind::UnsupportedFieldCombination)
        },
        Source::UnsupportedFieldForContext { field, context } => {
            field_context_with_context(Kind::UnsupportedFieldForContext, field, context)
        }
        Source::RequiredFieldForContext { context, field } => {
            field_context_with_context(Kind::RequiredFieldForContext, field, context)
        }
        Source::ZeroImageDimension { field } => field_context(Kind::ZeroImageDimension, field),
        Source::ImageEncodingFailed { field, source } => BridgeInvalidRequestContext {
            field: Some(field),
            source: Some(source.to_string()),
            ..invalid_request_context(Kind::ImageEncodingFailed)
        },
        _ => return None,
    };
    Some(context)
}

fn field_context(kind: BridgeInvalidRequestKind, field: String) -> BridgeInvalidRequestContext {
    BridgeInvalidRequestContext {
        field: Some(field),
        ..invalid_request_context(kind)
    }
}

fn field_context_with_context(
    kind: BridgeInvalidRequestKind,
    field: String,
    context: String,
) -> BridgeInvalidRequestContext {
    BridgeInvalidRequestContext {
        field: Some(field),
        context: Some(context),
        ..invalid_request_context(kind)
    }
}

const fn invalid_request_context(kind: BridgeInvalidRequestKind) -> BridgeInvalidRequestContext {
    BridgeInvalidRequestContext {
        kind,
        field: None,
        name: None,
        value: None,
        min: None,
        max: None,
        multiple_of: None,
        reason: None,
        source: None,
        feature: None,
        required_model: None,
        left: None,
        right: None,
        context: None,
    }
}

fn map_api_reason(reason: bridge::ApiErrorReason) -> BridgeApiErrorReason {
    match reason {
        bridge::ApiErrorReason::Message(value) => BridgeApiErrorReason::Message(value),
        bridge::ApiErrorReason::Detail(value) => BridgeApiErrorReason::Detail(value),
        bridge::ApiErrorReason::ErrorMessage(value) => BridgeApiErrorReason::ErrorMessage(value),
        other => BridgeApiErrorReason::Message(other.to_string()),
    }
}

const fn map_transport_operation(
    operation: bridge::TransportOperation,
) -> BridgeTransportOperation {
    match operation {
        bridge::TransportOperation::BuildClient => BridgeTransportOperation::BuildClient,
        bridge::TransportOperation::BuildHeader => BridgeTransportOperation::BuildHeader,
        bridge::TransportOperation::ReadResponseBytes => {
            BridgeTransportOperation::ReadResponseBytes
        }
        bridge::TransportOperation::ParseSse => BridgeTransportOperation::ParseSse,
        _ => BridgeTransportOperation::SendRequest,
    }
}

const fn map_decode_target(target: bridge::DecodeTarget) -> BridgeDecodeTarget {
    match target {
        bridge::DecodeTarget::JsonRequest => BridgeDecodeTarget::JsonRequest,
        bridge::DecodeTarget::StreamChunk => BridgeDecodeTarget::StreamChunk,
        bridge::DecodeTarget::ImageResponse => BridgeDecodeTarget::ImageResponse,
        _ => BridgeDecodeTarget::JsonResponse,
    }
}

fn map_metadata_context(error: bridge::MetadataError) -> Option<BridgeMetadataContext> {
    match error {
        bridge::MetadataError::InvalidPngPayload { field, source } => Some(BridgeMetadataContext {
            kind: BridgeMetadataKind::InvalidPngPayload,
            field,
            source: source.to_string(),
        }),
        _ => None,
    }
}
