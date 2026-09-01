use super::{
    BridgeApiErrorReason, BridgeDecodeTarget, BridgeInvalidRequestKind, BridgeMetadataKind,
    BridgeTransportOperation, DirectorApiErrorContext, DirectorApiErrorReason, DirectorClientError,
    DirectorDecodeContext, DirectorDecodeTarget, DirectorInvalidRequestContext,
    DirectorInvalidRequestKind, DirectorMetadataContext, DirectorMetadataKind,
    DirectorTransportContext, DirectorTransportOperation, GenerationApiErrorContext,
    GenerationApiErrorReason, GenerationClientError, GenerationDecodeContext,
    GenerationDecodeTarget, GenerationInvalidRequestContext, GenerationInvalidRequestKind,
    GenerationMetadataContext, GenerationMetadataKind, GenerationTransportContext,
    GenerationTransportOperation, NovelAiBridgeError, SubscriptionApiErrorContext,
    SubscriptionApiErrorReason, SubscriptionClientError, SubscriptionDecodeContext,
    SubscriptionDecodeTarget, SubscriptionInvalidRequestContext, SubscriptionInvalidRequestKind,
    SubscriptionMetadataContext, SubscriptionMetadataKind, SubscriptionTransportContext,
    SubscriptionTransportOperation, VibeApiErrorContext, VibeApiErrorReason, VibeClientError,
    VibeDecodeContext, VibeDecodeTarget, VibeInvalidRequestContext, VibeInvalidRequestKind,
    VibeMetadataContext, VibeMetadataKind, VibeTransportContext, VibeTransportOperation,
};

macro_rules! map_invalid_context {
    ($context:expr, $context_target:ident, $kind_target:ident) => {
        $context_target {
            kind: match $context.kind {
                BridgeInvalidRequestKind::EmptyField => $kind_target::EmptyField,
                BridgeInvalidRequestKind::MissingConfiguration => {
                    $kind_target::MissingConfiguration
                }
                BridgeInvalidRequestKind::NumericOutOfRange => $kind_target::NumericOutOfRange,
                BridgeInvalidRequestKind::InvalidImageDimension => {
                    $kind_target::InvalidImageDimension
                }
                BridgeInvalidRequestKind::NonFiniteNumber => $kind_target::NonFiniteNumber,
                BridgeInvalidRequestKind::InvalidDataUrl => $kind_target::InvalidDataUrl,
                BridgeInvalidRequestKind::InvalidBase64 => $kind_target::InvalidBase64,
                BridgeInvalidRequestKind::UndecodableImage => $kind_target::UndecodableImage,
                BridgeInvalidRequestKind::UnsupportedModelFeature => {
                    $kind_target::UnsupportedModelFeature
                }
                BridgeInvalidRequestKind::UnsupportedFieldCombination => {
                    $kind_target::UnsupportedFieldCombination
                }
                BridgeInvalidRequestKind::UnsupportedFieldForContext => {
                    $kind_target::UnsupportedFieldForContext
                }
                BridgeInvalidRequestKind::RequiredFieldForContext => {
                    $kind_target::RequiredFieldForContext
                }
                BridgeInvalidRequestKind::ZeroImageDimension => $kind_target::ZeroImageDimension,
                BridgeInvalidRequestKind::ImageEncodingFailed => $kind_target::ImageEncodingFailed,
            },
            field: $context.field,
            name: $context.name,
            value: $context.value,
            min: $context.min,
            max: $context.max,
            multiple_of: $context.multiple_of,
            reason: $context.reason,
            source: $context.source,
            feature: $context.feature,
            required_model: $context.required_model,
            left: $context.left,
            right: $context.right,
            context: $context.context,
        }
    };
}

macro_rules! map_api_context {
    ($context:expr, $context_target:ident, $reason_target:ident) => {
        $context_target {
            endpoint: $context.endpoint,
            server_reason: $context.server_reason.map(|reason| match reason {
                BridgeApiErrorReason::Message(value) => $reason_target::Message(value),
                BridgeApiErrorReason::Detail(value) => $reason_target::Detail(value),
                BridgeApiErrorReason::ErrorMessage(value) => $reason_target::ErrorMessage(value),
            }),
            raw_body: $context.raw_body,
        }
    };
}

macro_rules! map_transport_context {
    ($context:expr, $context_target:ident, $operation_target:ident) => {
        $context_target {
            operation: match $context.operation {
                BridgeTransportOperation::BuildClient => $operation_target::BuildClient,
                BridgeTransportOperation::BuildHeader => $operation_target::BuildHeader,
                BridgeTransportOperation::SendRequest => $operation_target::SendRequest,
                BridgeTransportOperation::ReadResponseBytes => $operation_target::ReadResponseBytes,
                BridgeTransportOperation::ParseSse => $operation_target::ParseSse,
            },
            endpoint: $context.endpoint,
            source: $context.source,
        }
    };
}

macro_rules! map_decode_context {
    ($context:expr, $context_target:ident, $target:ident) => {
        $context_target {
            target: match $context.target {
                BridgeDecodeTarget::JsonRequest => $target::JsonRequest,
                BridgeDecodeTarget::JsonResponse => $target::JsonResponse,
                BridgeDecodeTarget::StreamChunk => $target::StreamChunk,
                BridgeDecodeTarget::ImageResponse => $target::ImageResponse,
            },
            source: $context.source,
        }
    };
}

macro_rules! map_metadata_context {
    ($context:expr, $context_target:ident, $kind_target:ident) => {
        $context_target {
            kind: match $context.kind {
                BridgeMetadataKind::InvalidPngPayload => $kind_target::InvalidPngPayload,
            },
            field: $context.field,
            source: $context.source,
        }
    };
}

macro_rules! map_client_error {
    (
        $error:expr,
        $target:ty,
        $invalid_context:ident,
        $invalid_kind:ident,
        $api_context:ident,
        $api_reason:ident,
        $transport_context:ident,
        $transport_operation:ident,
        $decode_context:ident,
        $decode_target:ident,
        $metadata_context:ident,
        $metadata_kind:ident
    ) => {
        match $error {
            NovelAiBridgeError::Credential { message } => <$target>::credential(message),
            NovelAiBridgeError::InvalidRequest {
                status,
                context,
                api_context,
                message,
            } => <$target>::invalid_request_with_contexts(
                status,
                context
                    .map(|context| map_invalid_context!(*context, $invalid_context, $invalid_kind)),
                api_context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
            NovelAiBridgeError::Authentication {
                status,
                context,
                message,
            } => <$target>::authentication_with_context(
                status,
                context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
            NovelAiBridgeError::InsufficientCredit {
                status,
                context,
                message,
            } => <$target>::insufficient_credit_with_context(
                status,
                context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
            NovelAiBridgeError::RequestConflict {
                status,
                context,
                message,
            } => <$target>::request_conflict_with_context(
                status,
                context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
            NovelAiBridgeError::RateLimited {
                status,
                retry_after,
                context,
                message,
            } => <$target>::rate_limited_with_context(
                status,
                retry_after,
                context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
            NovelAiBridgeError::ServiceUnavailable {
                status,
                context,
                message,
            } => <$target>::service_unavailable_with_context(
                status,
                context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
            NovelAiBridgeError::Transport { context, message } => {
                <$target>::transport_with_context(
                    context.map(|context| {
                        map_transport_context!(*context, $transport_context, $transport_operation)
                    }),
                    message,
                )
            }
            NovelAiBridgeError::Decode { context, message } => <$target>::decode_with_context(
                context
                    .map(|context| map_decode_context!(*context, $decode_context, $decode_target)),
                message,
            ),
            NovelAiBridgeError::Metadata { context, message } => <$target>::metadata_with_context(
                context.map(|context| {
                    map_metadata_context!(*context, $metadata_context, $metadata_kind)
                }),
                message,
            ),
            NovelAiBridgeError::UnknownApi {
                status,
                context,
                message,
            } => <$target>::unknown_api_with_context(
                status,
                context.map(|context| map_api_context!(*context, $api_context, $api_reason)),
                message,
            ),
        }
    };
}

pub fn map_generation_error(error: NovelAiBridgeError) -> GenerationClientError {
    map_client_error!(
        error,
        GenerationClientError,
        GenerationInvalidRequestContext,
        GenerationInvalidRequestKind,
        GenerationApiErrorContext,
        GenerationApiErrorReason,
        GenerationTransportContext,
        GenerationTransportOperation,
        GenerationDecodeContext,
        GenerationDecodeTarget,
        GenerationMetadataContext,
        GenerationMetadataKind
    )
}

pub fn map_vibe_error(error: NovelAiBridgeError) -> VibeClientError {
    map_client_error!(
        error,
        VibeClientError,
        VibeInvalidRequestContext,
        VibeInvalidRequestKind,
        VibeApiErrorContext,
        VibeApiErrorReason,
        VibeTransportContext,
        VibeTransportOperation,
        VibeDecodeContext,
        VibeDecodeTarget,
        VibeMetadataContext,
        VibeMetadataKind
    )
}

pub fn map_director_error(error: NovelAiBridgeError) -> DirectorClientError {
    map_client_error!(
        error,
        DirectorClientError,
        DirectorInvalidRequestContext,
        DirectorInvalidRequestKind,
        DirectorApiErrorContext,
        DirectorApiErrorReason,
        DirectorTransportContext,
        DirectorTransportOperation,
        DirectorDecodeContext,
        DirectorDecodeTarget,
        DirectorMetadataContext,
        DirectorMetadataKind
    )
}

pub fn map_subscription_error(error: NovelAiBridgeError) -> SubscriptionClientError {
    map_client_error!(
        error,
        SubscriptionClientError,
        SubscriptionInvalidRequestContext,
        SubscriptionInvalidRequestKind,
        SubscriptionApiErrorContext,
        SubscriptionApiErrorReason,
        SubscriptionTransportContext,
        SubscriptionTransportOperation,
        SubscriptionDecodeContext,
        SubscriptionDecodeTarget,
        SubscriptionMetadataContext,
        SubscriptionMetadataKind
    )
}
