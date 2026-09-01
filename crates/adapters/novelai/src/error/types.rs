use std::time::Duration;

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
    JsonRequest,
    JsonResponse,
    StreamChunk,
    ImageResponse,
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
