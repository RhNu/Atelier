use async_trait::async_trait;
use atelier_resource_catalog::ResourceRef;
use thiserror::Error;

use crate::{
    CharacterPosition, CharacterReferenceType, ImageFormat, ImageModel, ImageSize, NoiseSchedule,
    Sampler, UcPreset,
};

pub type GenerationDraftResult<T> = Result<T, GenerationDraftError>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GenerationDraftErrorKind {
    InvalidValue,
    Repository,
}

impl std::fmt::Display for GenerationDraftErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidValue => "generation_draft_invalid_value",
            Self::Repository => "generation_draft_repository",
        })
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct GenerationDraftError {
    pub kind: GenerationDraftErrorKind,
    pub field: Option<String>,
    pub message: String,
}

impl GenerationDraftError {
    #[must_use]
    pub fn invalid(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: GenerationDraftErrorKind::InvalidValue,
            field: Some(field.into()),
            message: message.into(),
        }
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self {
            kind: GenerationDraftErrorKind::Repository,
            field: None,
            message: message.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GenerationDraftSeedMode {
    Random,
    Fixed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GenerationDraftCharacterPositionMode {
    Global,
    Manual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationDraftI2i {
    pub image: ResourceRef,
    pub mask: Option<ResourceRef>,
    pub strength: f32,
    pub noise: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationDraftVibeSlot {
    pub id: String,
    pub encoding: ResourceRef,
    pub vibe_id: Option<String>,
    pub information_extracted: f32,
    pub strength: f32,
    pub display_name: String,
    pub source_image: Option<ResourceRef>,
    pub source_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationDraftVibe {
    pub enabled: bool,
    pub strength: f32,
    pub slots: Vec<GenerationDraftVibeSlot>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationDraftPreciseReference {
    pub id: String,
    pub image: ResourceRef,
    pub reference_type: CharacterReferenceType,
    pub fidelity: f32,
    pub strength: f32,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationDraftCharacter {
    pub id: String,
    pub preset_id: Option<String>,
    pub prompt: String,
    pub negative_prompt: String,
    pub enabled: bool,
    pub position: CharacterPosition,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct GenerationDraftSnapshot {
    pub main_preset_id: Option<String>,
    pub prompt: String,
    pub negative_prompt: String,
    pub model: ImageModel,
    pub size: ImageSize,
    pub quality: bool,
    pub uc_preset: UcPreset,
    pub steps: u32,
    pub scale: f32,
    pub sampler: Sampler,
    pub noise_schedule: NoiseSchedule,
    pub seed_mode: GenerationDraftSeedMode,
    pub seed: i64,
    pub n_samples: u32,
    pub request_count: u32,
    pub cfg_rescale: f32,
    pub variety_boost: bool,
    pub image_format: Option<ImageFormat>,
    pub strict_mode: bool,
    pub stream_enabled: bool,
    pub i2i: Option<GenerationDraftI2i>,
    pub vibe: GenerationDraftVibe,
    pub precise_references: Vec<GenerationDraftPreciseReference>,
    pub characters: Vec<GenerationDraftCharacter>,
    pub character_position_mode: GenerationDraftCharacterPositionMode,
}

impl GenerationDraftSnapshot {
    /// Validates persisted generation workbench state before it reaches adapters or commands.
    ///
    /// # Errors
    /// Returns an error when numeric controls, stable ids, coordinates, or resource references are
    /// outside the ranges supported by the generation UI.
    pub fn validate(&self) -> GenerationDraftResult<()> {
        validate_optional_id("main_preset_id", self.main_preset_id.as_deref())?;
        validate_dimension("size.width", self.size.width)?;
        validate_dimension("size.height", self.size.height)?;
        validate_u32("steps", self.steps, 1, 50)?;
        validate_f32("scale", self.scale, 0.0, 10.0)?;
        validate_u32("n_samples", self.n_samples, 1, 4)?;
        validate_u32("request_count", self.request_count, 1, 8)?;
        validate_f32("cfg_rescale", self.cfg_rescale, 0.0, 1.0)?;

        if let Some(i2i) = &self.i2i {
            validate_resource("i2i.image", &i2i.image)?;
            if let Some(mask) = &i2i.mask {
                validate_resource("i2i.mask", mask)?;
            }
            validate_f32("i2i.strength", i2i.strength, 0.01, 0.99)?;
            validate_f32("i2i.noise", i2i.noise, 0.0, 0.99)?;
        }

        validate_f32("vibe.strength", self.vibe.strength, 0.0, 1.0)?;
        for (index, slot) in self.vibe.slots.iter().enumerate() {
            validate_id(&format!("vibe.slots[{index}].id"), &slot.id)?;
            validate_optional_id(
                &format!("vibe.slots[{index}].vibe_id"),
                slot.vibe_id.as_deref(),
            )?;
            validate_resource(&format!("vibe.slots[{index}].encoding"), &slot.encoding)?;
            if let Some(source) = &slot.source_image {
                validate_resource(&format!("vibe.slots[{index}].source_image"), source)?;
            }
            validate_f32(
                &format!("vibe.slots[{index}].information_extracted"),
                slot.information_extracted,
                0.01,
                1.0,
            )?;
            validate_f32(
                &format!("vibe.slots[{index}].strength"),
                slot.strength,
                0.0,
                1.0,
            )?;
        }

        for (index, reference) in self.precise_references.iter().enumerate() {
            validate_id(&format!("precise_references[{index}].id"), &reference.id)?;
            validate_resource(
                &format!("precise_references[{index}].image"),
                &reference.image,
            )?;
            validate_f32(
                &format!("precise_references[{index}].fidelity"),
                reference.fidelity,
                0.0,
                1.0,
            )?;
            validate_f32(
                &format!("precise_references[{index}].strength"),
                reference.strength,
                0.0,
                1.0,
            )?;
        }

        for (index, character) in self.characters.iter().enumerate() {
            validate_id(&format!("characters[{index}].id"), &character.id)?;
            validate_optional_id(
                &format!("characters[{index}].preset_id"),
                character.preset_id.as_deref(),
            )?;
            validate_f32(
                &format!("characters[{index}].position.x"),
                character.position.x,
                0.0,
                1.0,
            )?;
            validate_f32(
                &format!("characters[{index}].position.y"),
                character.position.y,
                0.0,
                1.0,
            )?;
        }

        if !self.precise_references.is_empty() && self.vibe.enabled && !self.vibe.slots.is_empty() {
            return Err(GenerationDraftError::invalid(
                "vibe",
                "Vibe transfer cannot be enabled while precise references are active",
            ));
        }
        Ok(())
    }
}

#[async_trait]
pub trait GenerationDraftRepository: Send + Sync {
    async fn load_generation_draft(&self)
    -> GenerationDraftResult<Option<GenerationDraftSnapshot>>;

    async fn save_generation_draft(
        &self,
        draft: &GenerationDraftSnapshot,
    ) -> GenerationDraftResult<()>;

    async fn clear_generation_draft(&self) -> GenerationDraftResult<()>;
}

#[derive(Clone, Debug)]
pub struct GenerationDraftService<R> {
    repository: R,
}

impl<R> GenerationDraftService<R> {
    #[must_use]
    pub const fn new(repository: R) -> Self {
        Self { repository }
    }
}

impl<R> GenerationDraftService<R>
where
    R: GenerationDraftRepository,
{
    /// Loads and validates the current workspace draft.
    ///
    /// # Errors
    /// Returns an error when persisted data cannot be decoded or is invalid.
    pub async fn load(&self) -> GenerationDraftResult<Option<GenerationDraftSnapshot>> {
        let draft = self.repository.load_generation_draft().await?;
        if let Some(value) = &draft {
            value.validate()?;
        }
        Ok(draft)
    }

    /// Validates and replaces the current workspace draft.
    ///
    /// # Errors
    /// Returns an error when the draft is invalid or persistence fails.
    pub async fn save(
        &self,
        draft: GenerationDraftSnapshot,
    ) -> GenerationDraftResult<GenerationDraftSnapshot> {
        draft.validate()?;
        self.repository.save_generation_draft(&draft).await?;
        Ok(draft)
    }

    /// Removes the current workspace draft.
    ///
    /// # Errors
    /// Returns an error when persistence fails.
    pub async fn clear(&self) -> GenerationDraftResult<()> {
        self.repository.clear_generation_draft().await
    }
}

fn validate_dimension(field: &str, value: u32) -> GenerationDraftResult<()> {
    if (64..=1600).contains(&value) && value.is_multiple_of(64) {
        Ok(())
    } else {
        Err(GenerationDraftError::invalid(
            field,
            format!("{field} must be between 64 and 1600 and divisible by 64"),
        ))
    }
}

fn validate_u32(field: &str, value: u32, min: u32, max: u32) -> GenerationDraftResult<()> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(GenerationDraftError::invalid(
            field,
            format!("{field} must be between {min} and {max}"),
        ))
    }
}

fn validate_f32(field: &str, value: f32, min: f32, max: f32) -> GenerationDraftResult<()> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(GenerationDraftError::invalid(
            field,
            format!("{field} must be finite and between {min} and {max}"),
        ))
    }
}

fn validate_id(field: &str, value: &str) -> GenerationDraftResult<()> {
    if value.trim().is_empty() {
        Err(GenerationDraftError::invalid(
            field,
            format!("{field} cannot be empty"),
        ))
    } else {
        Ok(())
    }
}

fn validate_optional_id(field: &str, value: Option<&str>) -> GenerationDraftResult<()> {
    if let Some(value) = value {
        validate_id(field, value)?;
    }
    Ok(())
}

fn validate_resource(field: &str, value: &ResourceRef) -> GenerationDraftResult<()> {
    validate_id(field, value.id.as_str())
}

#[cfg(test)]
mod tests;
