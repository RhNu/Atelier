use nai_atelier_generation::{CharacterReference, CharacterReferenceType};
use nai_atelier_resource_catalog::{ResourceKind, ResourceRef};
use thiserror::Error;

pub type PreciseReferenceResult<T> = Result<T, PreciseReferenceError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PreciseReferenceErrorKind {
    NotFound,
    InvalidResourceKind,
    EmptyPayload,
}

impl PreciseReferenceErrorKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
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
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(PreciseReferenceErrorKind::NotFound, message)
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

pub trait PreciseReferenceImageReader {
    /// Reads the image payload behind a precise-reference source resource.
    ///
    /// # Errors
    /// Returns an error when the resource cannot be resolved.
    fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreciseReferenceInput {
    pub source: ResourceRef,
    pub reference_type: CharacterReferenceType,
    pub fidelity: f32,
    pub strength: f32,
}

#[derive(Clone, Debug)]
pub struct PreciseReferenceService<R> {
    reader: R,
}

impl<R> PreciseReferenceService<R> {
    #[must_use]
    pub const fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R> PreciseReferenceService<R>
where
    R: PreciseReferenceImageReader,
{
    /// Prepares a resource-backed precise reference for `NovelAI` generation.
    ///
    /// The returned image payload is intentionally not resized or transformed;
    /// `novelai-bridge` owns the precise-reference preprocessing step.
    ///
    /// # Errors
    /// Returns an error when the source cannot be resolved, is not an image
    /// resource, or resolves to an empty payload.
    pub fn prepare(
        &self,
        input: &PreciseReferenceInput,
    ) -> PreciseReferenceResult<CharacterReference> {
        let image = self.reader.read_precise_reference_image(&input.source)?;
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
            reference_type: input.reference_type,
            fidelity: input.fidelity,
            strength: input.strength,
        })
    }
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
