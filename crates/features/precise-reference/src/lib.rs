use atelier_generation::{CharacterReference, CharacterReferenceType};
use atelier_resource_catalog::ResourceKind;
use thiserror::Error;

pub type PreciseReferenceResult<T> = Result<T, PreciseReferenceError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PreciseReferenceErrorKind {
    InvalidResourceKind,
    EmptyPayload,
}

impl PreciseReferenceErrorKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidResourceKind => "invalid_resource_kind",
            Self::EmptyPayload => "empty_payload",
        }
    }
}

impl std::fmt::Display for PreciseReferenceErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct PreciseReferenceError {
    kind: PreciseReferenceErrorKind,
    message: String,
}

impl PreciseReferenceError {
    #[must_use]
    pub fn new(kind: PreciseReferenceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn invalid_resource_kind(kind: ResourceKind) -> Self {
        Self::new(
            PreciseReferenceErrorKind::InvalidResourceKind,
            format!("resource kind `{kind:?}` cannot be used as a precise reference"),
        )
    }

    #[must_use]
    pub const fn kind(&self) -> PreciseReferenceErrorKind {
        self.kind
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreciseReferenceImage {
    pub kind: ResourceKind,
    pub payload: String,
}

/// Validates resolved reference input without reading or transforming image data.
/// `novelai-bridge` owns image preprocessing.
///
/// # Errors
/// Returns an error for a non-image resource or an empty payload.
pub fn prepare_reference(
    image: PreciseReferenceImage,
    reference_type: CharacterReferenceType,
    fidelity: f32,
    strength: f32,
) -> PreciseReferenceResult<CharacterReference> {
    if !is_precise_reference_image_kind(image.kind) {
        return Err(PreciseReferenceError::invalid_resource_kind(image.kind));
    }
    if image.payload.trim().is_empty() {
        return Err(PreciseReferenceError::new(
            PreciseReferenceErrorKind::EmptyPayload,
            "precise reference image payload cannot be empty",
        ));
    }
    Ok(CharacterReference {
        image: image.payload,
        reference_type,
        fidelity,
        strength,
    })
}

#[must_use]
pub const fn is_precise_reference_image_kind(kind: ResourceKind) -> bool {
    matches!(
        kind,
        ResourceKind::GeneratedImage
            | ResourceKind::StreamFinalImage
            | ResourceKind::DirectorResult
            | ResourceKind::SourceImage
            | ResourceKind::ReferenceImage
    )
}
