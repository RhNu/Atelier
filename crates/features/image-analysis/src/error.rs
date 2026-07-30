use thiserror::Error;

pub type ImageAnalysisResult<T> = Result<T, ImageAnalysisError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageAnalysisErrorKind {
    InvalidScore,
    InvalidRequest,
    ModelUnavailable,
    ModelInstall,
    Inference,
}

impl std::fmt::Display for ImageAnalysisErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidScore => "invalid_score",
            Self::InvalidRequest => "invalid_request",
            Self::ModelUnavailable => "model_unavailable",
            Self::ModelInstall => "model_install",
            Self::Inference => "inference",
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct ImageAnalysisError {
    kind: ImageAnalysisErrorKind,
    message: String,
}

impl ImageAnalysisError {
    #[must_use]
    pub fn invalid_score(message: impl Into<String>) -> Self {
        Self::new(ImageAnalysisErrorKind::InvalidScore, message)
    }

    #[must_use]
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(ImageAnalysisErrorKind::InvalidRequest, message)
    }

    #[must_use]
    pub fn model_unavailable(message: impl Into<String>) -> Self {
        Self::new(ImageAnalysisErrorKind::ModelUnavailable, message)
    }

    #[must_use]
    pub fn model_install(message: impl Into<String>) -> Self {
        Self::new(ImageAnalysisErrorKind::ModelInstall, message)
    }

    #[must_use]
    pub fn inference(message: impl Into<String>) -> Self {
        Self::new(ImageAnalysisErrorKind::Inference, message)
    }

    #[must_use]
    pub const fn kind(&self) -> ImageAnalysisErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(kind: ImageAnalysisErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}
