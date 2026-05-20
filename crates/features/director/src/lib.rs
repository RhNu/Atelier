use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;

pub type DirectorResult<T> = Result<T, DirectorClientError>;

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

impl ClientInvalidRequestContext {
    #[must_use]
    pub const fn new(kind: ClientInvalidRequestKind) -> Self {
        Self {
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
    JsonResponse,
    StreamChunk,
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
pub enum DirectorClientError {
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

impl DirectorClientError {
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum DirectorTool {
    #[default]
    Lineart,
    Sketch,
    BgRemoval,
    Emotion,
    Declutter,
    Colorize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDirectorToolRequest {
    pub tool: DirectorTool,
    pub image: String,
    pub prompt: Option<String>,
    pub defry: Option<u8>,
    pub strict_mode: bool,
}

impl RunDirectorToolRequest {
    /// Normalizes tool-specific fields to the shape accepted by `NovelAI` Director.
    ///
    /// Tools such as lineart and sketch do not accept prompt or defry options,
    /// while emotion requires a non-empty prompt. Colorize and emotion always
    /// send a resolved defry value to match `NovelAI`'s Director wire contract.
    /// # Errors
    /// Returns an invalid request error when required tool-specific input is
    /// missing, or when strict mode rejects an out-of-range defry value.
    pub fn normalize_for_tool(mut self) -> DirectorResult<Self> {
        let prompt = self
            .prompt
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        match self.tool {
            DirectorTool::Lineart
            | DirectorTool::Sketch
            | DirectorTool::BgRemoval
            | DirectorTool::Declutter => {
                self.prompt = None;
                self.defry = None;
            }
            DirectorTool::Colorize => {
                let defry = normalize_defry(self.defry, self.strict_mode)?;
                self.prompt = prompt;
                self.defry = Some(defry);
            }
            DirectorTool::Emotion => {
                let defry = normalize_defry(self.defry, self.strict_mode)?;
                let Some(prompt) = prompt else {
                    let mut context = ClientInvalidRequestContext::new(
                        ClientInvalidRequestKind::RequiredFieldForContext,
                    );
                    context.field = Some("prompt".to_owned());
                    context.context = Some("emotion".to_owned());
                    return Err(DirectorClientError::invalid_request_with_context(
                        None,
                        Some(context),
                        "emotion director tool requires a prompt",
                    ));
                };
                self.prompt = Some(prompt);
                self.defry = Some(defry);
            }
        }

        Ok(self)
    }
}

fn normalize_defry(value: Option<u8>, strict_mode: bool) -> DirectorResult<u8> {
    let defry = value.unwrap_or(0);
    if strict_mode && defry > 5 {
        let mut context =
            ClientInvalidRequestContext::new(ClientInvalidRequestKind::NumericOutOfRange);
        context.field = Some("defry".to_owned());
        context.value = Some(defry.to_string());
        context.min = Some("0".to_owned());
        context.max = Some("5".to_owned());
        return Err(DirectorClientError::invalid_request_with_context(
            None,
            Some(context),
            "defry must be between 0 and 5",
        ));
    }
    Ok(defry.min(5))
}

impl Default for RunDirectorToolRequest {
    fn default() -> Self {
        Self {
            tool: DirectorTool::default(),
            image: String::new(),
            prompt: None,
            defry: None,
            strict_mode: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectorToolOutput {
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
    pub seed: Option<i64>,
}

#[async_trait]
pub trait NovelAiDirectorClient: Send + Sync {
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-director");
    }

    #[test]
    fn run_director_tool_request_defaults_to_lineart() {
        let request = RunDirectorToolRequest::default();

        assert_eq!(request.tool, DirectorTool::Lineart);
        assert!(request.strict_mode);
    }

    #[test]
    fn normalizes_lineart_options_to_bridge_compatible_shape() {
        let request = RunDirectorToolRequest {
            tool: DirectorTool::Lineart,
            image: "image".to_owned(),
            prompt: Some(" clean lines ".to_owned()),
            defry: Some(9),
            strict_mode: true,
        }
        .normalize_for_tool()
        .unwrap();

        assert_eq!(request.prompt, None);
        assert_eq!(request.defry, None);
    }

    #[test]
    fn emotion_requires_non_empty_prompt() {
        let error = RunDirectorToolRequest {
            tool: DirectorTool::Emotion,
            image: "image".to_owned(),
            prompt: Some("   ".to_owned()),
            defry: None,
            strict_mode: true,
        }
        .normalize_for_tool()
        .unwrap_err();

        match error {
            DirectorClientError::InvalidRequest {
                context: Some(context),
                ..
            } => {
                assert_eq!(
                    context.kind,
                    ClientInvalidRequestKind::RequiredFieldForContext
                );
                assert_eq!(context.field.as_deref(), Some("prompt"));
                assert_eq!(context.context.as_deref(), Some("emotion"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn colorize_clamps_defry_when_not_strict() {
        let request = RunDirectorToolRequest {
            tool: DirectorTool::Colorize,
            image: "image".to_owned(),
            prompt: Some(" palette ".to_owned()),
            defry: Some(9),
            strict_mode: false,
        }
        .normalize_for_tool()
        .unwrap();

        assert_eq!(request.prompt.as_deref(), Some("palette"));
        assert_eq!(request.defry, Some(5));
    }
}
