use nai_atelier_app_api::error::ErrorEnvelopeDto;
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

impl From<nai_atelier_secrets::SecretsError> for AppError {
    fn from(error: nai_atelier_secrets::SecretsError) -> Self {
        Self::new(error.kind.to_string(), error.message)
    }
}

impl From<nai_atelier_secrets::ProbeApiKeyError> for AppError {
    fn from(error: nai_atelier_secrets::ProbeApiKeyError) -> Self {
        Self::new("probe_failed", error.to_string())
    }
}

impl From<nai_atelier_prompt_resources::PromptResourceError> for AppError {
    fn from(error: nai_atelier_prompt_resources::PromptResourceError) -> Self {
        let code = match error.kind() {
            nai_atelier_prompt_resources::PromptResourceErrorKind::InvalidRequest => {
                "prompt_invalid_request"
            }
            nai_atelier_prompt_resources::PromptResourceErrorKind::NotFound => "prompt_not_found",
            nai_atelier_prompt_resources::PromptResourceErrorKind::Conflict => "prompt_conflict",
            nai_atelier_prompt_resources::PromptResourceErrorKind::Repository => {
                "prompt_repository"
            }
        };
        Self::new(code, error.to_string())
    }
}

impl From<nai_atelier_prompt_lexicon::PromptLexiconError> for AppError {
    fn from(error: nai_atelier_prompt_lexicon::PromptLexiconError) -> Self {
        Self::new("prompt_lexicon", error.to_string())
    }
}

impl From<nai_atelier_kernel::KernelError> for AppError {
    fn from(error: nai_atelier_kernel::KernelError) -> Self {
        Self::new("kernel", error.to_string())
    }
}

impl From<nai_atelier_gallery::GalleryError> for AppError {
    fn from(error: nai_atelier_gallery::GalleryError) -> Self {
        Self::new("gallery", error.to_string())
    }
}

impl From<nai_atelier_workspace::WorkspaceError> for AppError {
    fn from(error: nai_atelier_workspace::WorkspaceError) -> Self {
        Self::new(error.kind.to_string(), error.to_string())
    }
}

impl From<nai_atelier_adapter_database::DatabaseError> for AppError {
    fn from(error: nai_atelier_adapter_database::DatabaseError) -> Self {
        Self::new("database", error.to_string())
    }
}

impl From<nai_atelier_resource_catalog::ResourceCatalogError> for AppError {
    fn from(error: nai_atelier_resource_catalog::ResourceCatalogError) -> Self {
        Self::new(error.kind.to_string(), error.message)
    }
}

impl From<nai_atelier_settings::SettingsError> for AppError {
    fn from(error: nai_atelier_settings::SettingsError) -> Self {
        Self::new(error.kind.to_string(), error.message)
    }
}

impl From<nai_atelier_vibe::VibeError> for AppError {
    fn from(error: nai_atelier_vibe::VibeError) -> Self {
        Self::new(error.kind().to_string(), error.to_string())
    }
}

impl From<base64::DecodeError> for AppError {
    fn from(error: base64::DecodeError) -> Self {
        Self::new("invalid_base64", error.to_string())
    }
}
