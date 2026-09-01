use std::time::Duration;

use thiserror::Error;

pub type SecretsResult<T> = Result<T, SecretsError>;
pub type ProbeApiKeyResult<T> = Result<T, ProbeApiKeyError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretsErrorKind {
    Validation,
    MetadataStore,
    SecretStore,
    MissingActiveKey,
    MissingSecret,
}

impl std::fmt::Display for SecretsErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Validation => "validation",
            Self::MetadataStore => "metadata_store",
            Self::SecretStore => "secret_store",
            Self::MissingActiveKey => "missing_active_key",
            Self::MissingSecret => "missing_secret",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct SecretsError {
    pub kind: SecretsErrorKind,
    pub message: String,
}

impl SecretsError {
    #[must_use]
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(SecretsErrorKind::Validation, message)
    }

    #[must_use]
    pub fn metadata_store(message: impl Into<String>) -> Self {
        Self::new(SecretsErrorKind::MetadataStore, message)
    }

    #[must_use]
    pub fn secret_store(message: impl Into<String>) -> Self {
        Self::new(SecretsErrorKind::SecretStore, message)
    }

    #[must_use]
    pub fn missing_active_key() -> Self {
        Self::new(SecretsErrorKind::MissingActiveKey, "no active API key")
    }

    #[must_use]
    pub fn missing_secret(id: impl std::fmt::Display) -> Self {
        Self::new(
            SecretsErrorKind::MissingSecret,
            format!("missing secret `{id}`"),
        )
    }

    #[must_use]
    pub fn new(kind: SecretsErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ProbeApiKeyError {
    #[error("secret resolution failed: {0}")]
    Secrets(#[from] SecretsError),
    #[error("subscription probe failed: {0}")]
    Subscription(#[from] SubscriptionClientError),
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
pub enum SubscriptionClientError {
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

impl SubscriptionClientError {
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
