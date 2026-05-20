use serde::{Deserialize, Serialize};

use crate::generation::{
    ImageFormatDto, ImageModelDto, ImageSizeDto, NoiseScheduleDto, SamplerDto, UcPresetDto,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSettingsDto {
    pub generation: GenerationDefaultsDto,
    pub image_variants: ImageVariantSettingsDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenerationDefaultsDto {
    pub model: ImageModelDto,
    pub size: ImageSizeDto,
    pub quality: bool,
    pub uc_preset: UcPresetDto,
    pub steps: u32,
    pub scale: f32,
    pub sampler: SamplerDto,
    pub noise_schedule: NoiseScheduleDto,
    pub seed: i64,
    pub n_samples: u32,
    pub cfg_rescale: f32,
    pub variety_boost: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_format: Option<ImageFormatDto>,
    pub strict_mode: bool,
}

impl Default for GenerationDefaultsDto {
    fn default() -> Self {
        Self {
            model: ImageModelDto::default(),
            size: ImageSizeDto::default(),
            quality: true,
            uc_preset: UcPresetDto::default(),
            steps: 23,
            scale: 5.0,
            sampler: SamplerDto::default(),
            noise_schedule: NoiseScheduleDto::default(),
            seed: 0,
            n_samples: 1,
            cfg_rescale: 0.0,
            variety_boost: false,
            image_format: None,
            strict_mode: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageVariantSettingsDto {
    pub thumbnail_long_edge: u32,
    pub preview_long_edge: u32,
}

impl Default for ImageVariantSettingsDto {
    fn default() -> Self {
        Self {
            thumbnail_long_edge: 320,
            preview_long_edge: 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UpdateWorkspaceSettingsRequestDto {
    pub settings: WorkspaceSettingsDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResetWorkspaceSettingsResponseDto {
    pub settings: WorkspaceSettingsDto,
}
