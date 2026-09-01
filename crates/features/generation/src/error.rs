use std::time::Duration;

use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenerationErrorKind {
    EmptyField,
    NumericOutOfRange,
    NonFiniteNumber,
    InvalidImageDimension,
    UnsupportedModelFeature,
    UnsupportedFieldCombination,
}

impl std::fmt::Display for GenerationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::EmptyField => "empty_field",
            Self::NumericOutOfRange => "numeric_out_of_range",
            Self::NonFiniteNumber => "non_finite_number",
            Self::InvalidImageDimension => "invalid_image_dimension",
            Self::UnsupportedModelFeature => "unsupported_model_feature",
            Self::UnsupportedFieldCombination => "unsupported_field_combination",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct GenerationError {
    pub kind: GenerationErrorKind,
    pub field: Option<String>,
    pub message: String,
}

impl GenerationError {
    #[must_use]
    pub fn empty_field(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::EmptyField,
            Some(field.clone()),
            format!("{field} cannot be empty"),
        )
    }

    #[must_use]
    pub fn numeric_out_of_range(
        field: impl Into<String>,
        min: impl std::fmt::Display,
        max: impl std::fmt::Display,
    ) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::NumericOutOfRange,
            Some(field.clone()),
            format!("{field} must be between {min} and {max}"),
        )
    }

    #[must_use]
    pub fn non_finite_number(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::NonFiniteNumber,
            Some(field.clone()),
            format!("{field} must be finite"),
        )
    }

    #[must_use]
    pub fn invalid_image_dimension(field: impl Into<String>) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::InvalidImageDimension,
            Some(field.clone()),
            format!("{field} must be in [64,1600] and a multiple of 64"),
        )
    }

    #[must_use]
    pub fn unsupported_model_feature(
        field: impl Into<String>,
        required_model: impl std::fmt::Display,
    ) -> Self {
        let field = field.into();
        Self::new(
            GenerationErrorKind::UnsupportedModelFeature,
            Some(field.clone()),
            format!("{field} requires {required_model}"),
        )
    }

    #[must_use]
    pub fn unsupported_field_combination(
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            GenerationErrorKind::UnsupportedFieldCombination,
            Some(field.into()),
            message,
        )
    }

    #[must_use]
    pub fn new(
        kind: GenerationErrorKind,
        field: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            field,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientInvalidRequestContext {
    pub kind: ClientInvalidRequestKind,
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
pub enum ClientInvalidRequestKind {
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
pub struct ClientApiErrorContext {
    pub endpoint: String,
    pub server_reason: Option<ClientApiErrorReason>,
    pub raw_body: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientApiErrorReason {
    Message(String),
    Detail(String),
    ErrorMessage(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientTransportContext {
    pub operation: ClientTransportOperation,
    pub endpoint: Option<String>,
    pub source: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClientTransportOperation {
    BuildClient,
    BuildHeader,
    SendRequest,
    ReadResponseBytes,
    ParseSse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientDecodeContext {
    pub target: ClientDecodeTarget,
    pub source: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClientDecodeTarget {
    JsonRequest,
    JsonResponse,
    StreamChunk,
    ImageResponse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientMetadataContext {
    pub kind: ClientMetadataKind,
    pub field: String,
    pub source: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ClientMetadataKind {
    InvalidPngPayload,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum GenerationClientError {
    #[error("credential error: {message}")]
    Credential { message: String },
    #[error("invalid request: {message}")]
    InvalidRequest {
        status: Option<u16>,
        context: Option<Box<ClientInvalidRequestContext>>,
        api_context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
    #[error("authentication failed: {message}")]
    Authentication {
        status: Option<u16>,
        context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
    #[error("insufficient credit: {message}")]
    InsufficientCredit {
        status: Option<u16>,
        context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
    #[error("request conflict: {message}")]
    RequestConflict {
        status: Option<u16>,
        context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
    #[error("rate limited: {message}")]
    RateLimited {
        status: u16,
        retry_after: Option<Duration>,
        context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
    #[error("service unavailable: {message}")]
    ServiceUnavailable {
        status: Option<u16>,
        context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
    #[error("transport failed: {message}")]
    Transport {
        context: Option<Box<ClientTransportContext>>,
        message: String,
    },
    #[error("decode failed: {message}")]
    Decode {
        context: Option<Box<ClientDecodeContext>>,
        message: String,
    },
    #[error("metadata failed: {message}")]
    Metadata {
        context: Option<Box<ClientMetadataContext>>,
        message: String,
    },
    #[error("unknown api error: {message}")]
    UnknownApi {
        status: Option<u16>,
        context: Option<Box<ClientApiErrorContext>>,
        message: String,
    },
}

fn boxed<T>(value: Option<T>) -> Option<Box<T>> {
    value.map(Box::new)
}

impl GenerationClientError {
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
        context: Option<ClientInvalidRequestContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::invalid_request_with_contexts(status, context, None, message)
    }

    #[must_use]
    pub fn invalid_request_with_contexts(
        status: Option<u16>,
        context: Option<ClientInvalidRequestContext>,
        api_context: Option<ClientApiErrorContext>,
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
        context: Option<ClientApiErrorContext>,
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
        context: Option<ClientApiErrorContext>,
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
        context: Option<ClientApiErrorContext>,
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
        context: Option<ClientApiErrorContext>,
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
        context: Option<ClientApiErrorContext>,
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
        context: Option<ClientTransportContext>,
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
        context: Option<ClientDecodeContext>,
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
        context: Option<ClientMetadataContext>,
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
        context: Option<ClientApiErrorContext>,
        message: impl Into<String>,
    ) -> Self {
        Self::UnknownApi {
            status,
            context: boxed(context),
            message: message.into(),
        }
    }
}
