use atelier_app_api::error::ErrorEnvelopeDto;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{code}: {message}")]
pub struct AppError {
    code: String,
    message: String,
}

impl AppError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn missing_active_key() -> Self {
        Self::new("missing_active_key", "no active NovelAI API key")
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub fn envelope(&self) -> ErrorEnvelopeDto {
        ErrorEnvelopeDto::new(self.code.clone(), self.message.clone())
    }
}

impl From<atelier_secrets::SecretsError> for AppError {
    fn from(error: atelier_secrets::SecretsError) -> Self {
        Self::new(error.kind.to_string(), error.message)
    }
}

impl From<atelier_secrets::ProbeApiKeyError> for AppError {
    fn from(error: atelier_secrets::ProbeApiKeyError) -> Self {
        Self::new("probe_failed", error.to_string())
    }
}

impl From<atelier_prompt_resources::PromptResourceError> for AppError {
    fn from(error: atelier_prompt_resources::PromptResourceError) -> Self {
        let code = match error.kind() {
            atelier_prompt_resources::PromptResourceErrorKind::InvalidRequest => {
                "prompt_invalid_request"
            }
            atelier_prompt_resources::PromptResourceErrorKind::NotFound => "prompt_not_found",
            atelier_prompt_resources::PromptResourceErrorKind::Conflict => "prompt_conflict",
            atelier_prompt_resources::PromptResourceErrorKind::Repository => "prompt_repository",
        };
        Self::new(code, error.to_string())
    }
}

impl From<atelier_prompt_lexicon::LexiconError> for AppError {
    fn from(error: atelier_prompt_lexicon::LexiconError) -> Self {
        let code = match &error {
            atelier_prompt_lexicon::LexiconError::Unavailable(_) => "lexicon_unavailable",
            atelier_prompt_lexicon::LexiconError::InvalidRequest(_) => "lexicon_invalid_request",
            atelier_prompt_lexicon::LexiconError::NotFound(_) => "lexicon_not_found",
            atelier_prompt_lexicon::LexiconError::InvalidBundle(_) => "lexicon_invalid_bundle",
            atelier_prompt_lexicon::LexiconError::Query(_) => "lexicon_query",
            atelier_prompt_lexicon::LexiconError::SemanticUnavailable(_) => {
                "lexicon_semantic_unavailable"
            }
        };
        Self::new(code, error.to_string())
    }
}

impl From<atelier_kernel::KernelError> for AppError {
    fn from(error: atelier_kernel::KernelError) -> Self {
        Self::new("kernel", error.to_string())
    }
}

impl From<atelier_gallery::GalleryError> for AppError {
    fn from(error: atelier_gallery::GalleryError) -> Self {
        Self::new("gallery", error.to_string())
    }
}

impl From<atelier_image_analysis::ImageAnalysisError> for AppError {
    fn from(error: atelier_image_analysis::ImageAnalysisError) -> Self {
        Self::new(error.kind().to_string(), error.message())
    }
}

impl From<atelier_generation::GenerationDraftError> for AppError {
    fn from(error: atelier_generation::GenerationDraftError) -> Self {
        Self::new(error.kind.to_string(), error.to_string())
    }
}

impl From<atelier_workspace::WorkspaceError> for AppError {
    fn from(error: atelier_workspace::WorkspaceError) -> Self {
        Self::new(error.kind.to_string(), error.to_string())
    }
}

impl From<atelier_adapter_database::DatabaseError> for AppError {
    fn from(error: atelier_adapter_database::DatabaseError) -> Self {
        let code = match error.kind() {
            atelier_adapter_database::DatabaseErrorKind::Database => "database",
            atelier_adapter_database::DatabaseErrorKind::UnsupportedSchema => "unsupported_schema",
        };
        Self::new(code, error.to_string())
    }
}

impl From<atelier_resource_catalog::ResourceCatalogError> for AppError {
    fn from(error: atelier_resource_catalog::ResourceCatalogError) -> Self {
        Self::new(error.kind.to_string(), error.message)
    }
}

impl From<atelier_settings::SettingsError> for AppError {
    fn from(error: atelier_settings::SettingsError) -> Self {
        Self::new(error.kind.to_string(), error.message)
    }
}

impl From<atelier_vibe::VibeError> for AppError {
    fn from(error: atelier_vibe::VibeError) -> Self {
        Self::new(error.kind().to_string(), error.to_string())
    }
}

impl From<base64::DecodeError> for AppError {
    fn from(error: base64::DecodeError) -> Self {
        Self::new("invalid_base64", error.to_string())
    }
}
