use std::path::PathBuf;

use atelier_generation::{ImageFormat, ImageModel, ImageSize, NoiseSchedule, Sampler, UcPreset};

use crate::{SettingsError, SettingsResult};

const VARIANT_LONG_EDGE_MAX: u32 = 4096;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkspaceSettings {
    pub generation: GenerationDefaults,
    pub image_variants: ImageVariantSettings,
}

impl WorkspaceSettings {
    /// Validates workspace settings before persistence or use.
    ///
    /// # Errors
    /// Returns an error when scalar generation defaults or image variant limits
    /// are outside the supported v1 range.
    pub fn validate(&self) -> SettingsResult<()> {
        self.generation.validate()?;
        self.image_variants.validate()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationDefaults {
    pub model: ImageModel,
    pub size: ImageSize,
    pub quality: bool,
    pub uc_preset: UcPreset,
    pub steps: u32,
    pub scale: f32,
    pub sampler: Sampler,
    pub noise_schedule: NoiseSchedule,
    pub seed: i64,
    pub n_samples: u32,
    pub cfg_rescale: f32,
    pub variety_boost: bool,
    pub image_format: Option<ImageFormat>,
    pub strict_mode: bool,
}

impl Default for GenerationDefaults {
    fn default() -> Self {
        Self {
            model: ImageModel::default(),
            size: ImageSize::default(),
            quality: true,
            uc_preset: UcPreset::default(),
            steps: 23,
            scale: 5.0,
            sampler: Sampler::default(),
            noise_schedule: NoiseSchedule::default(),
            seed: 0,
            n_samples: 1,
            cfg_rescale: 0.0,
            variety_boost: false,
            image_format: None,
            strict_mode: false,
        }
    }
}

impl GenerationDefaults {
    fn validate(&self) -> SettingsResult<()> {
        ensure_u32_range("generation.steps", self.steps, 1, 50)?;
        ensure_u32_range("generation.n_samples", self.n_samples, 1, 4)?;
        ensure_f32_range("generation.scale", self.scale, 0.0, 10.0)?;
        ensure_f32_range("generation.cfg_rescale", self.cfg_rescale, 0.0, 1.0)?;
        ensure_image_dimension("generation.size.width", self.size.width)?;
        ensure_image_dimension("generation.size.height", self.size.height)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ImageVariantSettings {
    pub thumbnail_long_edge: u32,
    pub preview_long_edge: u32,
}

impl Default for ImageVariantSettings {
    fn default() -> Self {
        Self {
            thumbnail_long_edge: 320,
            preview_long_edge: 1024,
        }
    }
}

impl ImageVariantSettings {
    fn validate(self) -> SettingsResult<()> {
        ensure_u32_range(
            "image_variants.thumbnail_long_edge",
            self.thumbnail_long_edge,
            1,
            VARIANT_LONG_EDGE_MAX,
        )?;
        ensure_u32_range(
            "image_variants.preview_long_edge",
            self.preview_long_edge,
            1,
            VARIANT_LONG_EDGE_MAX,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalSettings {
    pub last_workspace: Option<PathBuf>,
    pub frontend: GlobalFrontendSettings,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalFrontendSettings {
    pub gallery: GlobalGallerySettings,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct GlobalGallerySettings {
    pub blur_sensitive_images: bool,
}

fn ensure_u32_range(field: &str, value: u32, min: u32, max: u32) -> SettingsResult<()> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(SettingsError::invalid_value(format!(
            "{field} must be between {min} and {max}"
        )))
    }
}

fn ensure_f32_range(field: &str, value: f32, min: f32, max: f32) -> SettingsResult<()> {
    if value.is_finite() && (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(SettingsError::invalid_value(format!(
            "{field} must be finite and between {min} and {max}"
        )))
    }
}

fn ensure_image_dimension(field: &str, value: u32) -> SettingsResult<()> {
    if (64..=1600).contains(&value) && value.is_multiple_of(64) {
        Ok(())
    } else {
        Err(SettingsError::invalid_value(format!(
            "{field} must be between 64 and 1600 and divisible by 64"
        )))
    }
}
