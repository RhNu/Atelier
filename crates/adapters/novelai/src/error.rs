use std::time::Duration;

use nai_atelier_director::{
    ClientApiErrorContext as DirectorApiErrorContext,
    ClientApiErrorReason as DirectorApiErrorReason, ClientDecodeContext as DirectorDecodeContext,
    ClientDecodeTarget as DirectorDecodeTarget,
    ClientInvalidRequestContext as DirectorInvalidRequestContext,
    ClientInvalidRequestKind as DirectorInvalidRequestKind,
    ClientMetadataContext as DirectorMetadataContext, ClientMetadataKind as DirectorMetadataKind,
    ClientTransportContext as DirectorTransportContext,
    ClientTransportOperation as DirectorTransportOperation, DirectorClientError,
};
use nai_atelier_generation::{
    ClientApiErrorContext as GenerationApiErrorContext,
    ClientApiErrorReason as GenerationApiErrorReason,
    ClientDecodeContext as GenerationDecodeContext, ClientDecodeTarget as GenerationDecodeTarget,
    ClientInvalidRequestContext as GenerationInvalidRequestContext,
    ClientInvalidRequestKind as GenerationInvalidRequestKind,
    ClientMetadataContext as GenerationMetadataContext,
    ClientMetadataKind as GenerationMetadataKind,
    ClientTransportContext as GenerationTransportContext,
    ClientTransportOperation as GenerationTransportOperation, GenerationClientError,
};
use nai_atelier_secrets::{
    ClientApiErrorContext as SubscriptionApiErrorContext,
    ClientApiErrorReason as SubscriptionApiErrorReason,
    ClientDecodeContext as SubscriptionDecodeContext,
    ClientDecodeTarget as SubscriptionDecodeTarget,
    ClientInvalidRequestContext as SubscriptionInvalidRequestContext,
    ClientInvalidRequestKind as SubscriptionInvalidRequestKind,
    ClientMetadataContext as SubscriptionMetadataContext,
    ClientMetadataKind as SubscriptionMetadataKind,
    ClientTransportContext as SubscriptionTransportContext,
    ClientTransportOperation as SubscriptionTransportOperation, SubscriptionClientError,
};
use nai_atelier_vibe::{
    ClientApiErrorContext as VibeApiErrorContext, ClientApiErrorReason as VibeApiErrorReason,
    ClientDecodeContext as VibeDecodeContext, ClientDecodeTarget as VibeDecodeTarget,
    ClientInvalidRequestContext as VibeInvalidRequestContext,
    ClientInvalidRequestKind as VibeInvalidRequestKind,
    ClientMetadataContext as VibeMetadataContext, ClientMetadataKind as VibeMetadataKind,
    ClientTransportContext as VibeTransportContext,
    ClientTransportOperation as VibeTransportOperation, VibeClientError,
};
use novelai_bridge as bridge;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeInvalidRequestContext {
    pub kind: BridgeInvalidRequestKind,
    pub field: Option<String>,
    pub name: Option<String>,
    pub value: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub multiple_of: Option<u32>,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub feature: Option<String>,
    pub required_model: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub context: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeInvalidRequestKind {
    EmptyField,
    MissingConfiguration,
    NumericOutOfRange,
    InvalidImageDimension,
    NonFiniteNumber,
    InvalidDataUrl,
    InvalidBase64,
    UndecodableImage,
    UnsupportedModelFeature,
    UnsupportedFieldCombination,
    UnsupportedFieldForContext,
    RequiredFieldForContext,
    ZeroImageDimension,
    ImageEncodingFailed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeApiErrorContext {
    pub endpoint: String,
    pub server_reason: Option<BridgeApiErrorReason>,
    pub raw_body: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeApiErrorReason {
    Message(String),
    Detail(String),
    ErrorMessage(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeTransportContext {
    pub operation: BridgeTransportOperation,
    pub endpoint: Option<String>,
    pub source: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeTransportOperation {
    BuildClient,
    BuildHeader,
    SendRequest,
    ReadResponseBytes,
    ParseSse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeDecodeContext {
    pub target: BridgeDecodeTarget,
    pub source: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeDecodeTarget {
    JsonResponse,
    StreamChunk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeMetadataContext {
    pub kind: BridgeMetadataKind,
    pub field: String,
    pub source: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BridgeMetadataKind {
    InvalidPngPayload,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum NovelAiBridgeError {
    #[error("credential error: {message}")]
    Credential { message: String },
    #[error("invalid request: {message}")]
    InvalidRequest {
        status: Option<u16>,
        context: Option<Box<BridgeInvalidRequestContext>>,
        api_context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
    #[error("authentication failed: {message}")]
    Authentication {
        status: Option<u16>,
        context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
    #[error("insufficient credit: {message}")]
    InsufficientCredit {
        status: Option<u16>,
        context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
    #[error("request conflict: {message}")]
    RequestConflict {
        status: Option<u16>,
        context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
    #[error("rate limited: {message}")]
    RateLimited {
        status: u16,
        retry_after: Option<Duration>,
        context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
    #[error("service unavailable: {message}")]
    ServiceUnavailable {
        status: Option<u16>,
        context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
    #[error("transport failed: {message}")]
    Transport {
        context: Option<Box<BridgeTransportContext>>,
        message: String,
    },
    #[error("decode failed: {message}")]
    Decode {
        context: Option<Box<BridgeDecodeContext>>,
        message: String,
    },
    #[error("metadata failed: {message}")]
    Metadata {
        context: Option<Box<BridgeMetadataContext>>,
        message: String,
    },
    #[error("unknown api error: {message}")]
    UnknownApi {
        status: Option<u16>,
        context: Option<Box<BridgeApiErrorContext>>,
        message: String,
    },
}

fn boxed<T>(value: Option<T>) -> Option<Box<T>> {
    value.map(Box::new)
}

impl NovelAiBridgeError {
    #[must_use]
    pub fn credential(message: impl Into<String>) -> Self {
        Self::Credential {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn invalid_request(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::invalid_request_with_contexts(status, None, None, message)
    }

    #[must_use]
    pub fn invalid_request_with_context(
        status: Option<u16>,
        context: Option<BridgeInvalidRequestContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::invalid_request_with_contexts(status, context, None, message)
    }

    #[must_use]
    pub fn invalid_request_with_contexts(
        status: Option<u16>,
        context: Option<BridgeInvalidRequestContext>,
        api_context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidRequest {
            status,
            context: boxed(context),
            api_context: boxed(api_context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn authentication(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::authentication_with_context(status, None, message)
    }

    #[must_use]
    pub fn authentication_with_context(
        status: Option<u16>,
        context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::Authentication {
            status,
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn insufficient_credit(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::insufficient_credit_with_context(status, None, message)
    }

    #[must_use]
    pub fn insufficient_credit_with_context(
        status: Option<u16>,
        context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::InsufficientCredit {
            status,
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn request_conflict(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::request_conflict_with_context(status, None, message)
    }

    #[must_use]
    pub fn request_conflict_with_context(
        status: Option<u16>,
        context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::RequestConflict {
            status,
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn rate_limited(
        status: u16,
        retry_after: Option<Duration>,
        message: impl Into<String>,
    ) -> Self {
        Self::rate_limited_with_context(status, retry_after, None, message)
    }

    #[must_use]
    pub fn rate_limited_with_context(
        status: u16,
        retry_after: Option<Duration>,
        context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::RateLimited {
            status,
            retry_after,
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn service_unavailable(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::service_unavailable_with_context(status, None, message)
    }

    #[must_use]
    pub fn service_unavailable_with_context(
        status: Option<u16>,
        context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::ServiceUnavailable {
            status,
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn transport(message: impl Into<String>) -> Self {
        Self::transport_with_context(None, message)
    }

    #[must_use]
    pub fn transport_with_context(
        context: Option<BridgeTransportContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::Transport {
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn decode(message: impl Into<String>) -> Self {
        Self::decode_with_context(None, message)
    }

    #[must_use]
    pub fn decode_with_context(
        context: Option<BridgeDecodeContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::Decode {
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn metadata(message: impl Into<String>) -> Self {
        Self::metadata_with_context(None, message)
    }

    #[must_use]
    pub fn metadata_with_context(
        context: Option<BridgeMetadataContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::Metadata {
            context: boxed(context),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn unknown_api(status: Option<u16>, message: impl Into<String>) -> Self {
        Self::unknown_api_with_context(status, None, message)
    }

    #[must_use]
    pub fn unknown_api_with_context(
        status: Option<u16>,
        context: Option<BridgeApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::UnknownApi {
            status,
            context: boxed(context),
            message: message.into(),
        }
    }
}

pub fn map_bridge_error(error: bridge::BridgeError) -> NovelAiBridgeError {
    match error {
        bridge::BridgeError::InvalidRequest(error) => {
            let message = error.to_string();
            let context = map_invalid_request_context(error);
            NovelAiBridgeError::invalid_request_with_contexts(None, Some(context), None, message)
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
            NovelAiBridgeError::metadata_with_context(Some(context), message)
        }
    }
}

fn map_invalid_request_context(error: bridge::InvalidRequest) -> BridgeInvalidRequestContext {
    use BridgeInvalidRequestKind as Kind;
    use bridge::InvalidRequest as Source;

    match error {
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
    }
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
    }
}

const fn map_transport_operation(
    operation: bridge::TransportOperation,
) -> BridgeTransportOperation {
    match operation {
        bridge::TransportOperation::BuildClient => BridgeTransportOperation::BuildClient,
        bridge::TransportOperation::BuildHeader => BridgeTransportOperation::BuildHeader,
        bridge::TransportOperation::SendRequest => BridgeTransportOperation::SendRequest,
        bridge::TransportOperation::ReadResponseBytes => {
            BridgeTransportOperation::ReadResponseBytes
        }
        bridge::TransportOperation::ParseSse => BridgeTransportOperation::ParseSse,
    }
}

const fn map_decode_target(target: bridge::DecodeTarget) -> BridgeDecodeTarget {
    match target {
        bridge::DecodeTarget::JsonResponse => BridgeDecodeTarget::JsonResponse,
        bridge::DecodeTarget::StreamChunk => BridgeDecodeTarget::StreamChunk,
    }
}

fn map_metadata_context(error: bridge::MetadataError) -> BridgeMetadataContext {
    match error {
        bridge::MetadataError::InvalidPngPayload { field, source } => BridgeMetadataContext {
            kind: BridgeMetadataKind::InvalidPngPayload,
            field,
            source: source.to_string(),
        },
    }
}

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
                BridgeDecodeTarget::JsonResponse => $target::JsonResponse,
                BridgeDecodeTarget::StreamChunk => $target::StreamChunk,
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
