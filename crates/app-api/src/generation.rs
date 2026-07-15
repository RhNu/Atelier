use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::resource::{ImageInputDto, ResourceRefDto};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ImageModelDto {
    #[default]
    #[serde(rename = "nai-diffusion-4-5-full")]
    NaiDiffusion45Full,
    #[serde(rename = "nai-diffusion-4-5-curated")]
    NaiDiffusion45Curated,
    #[serde(rename = "nai-diffusion-4-full")]
    NaiDiffusion4Full,
    #[serde(rename = "nai-diffusion-4-curated")]
    NaiDiffusion4Curated,
    #[serde(rename = "nai-diffusion-3")]
    NaiDiffusion3,
    #[serde(rename = "nai-diffusion-3-furry")]
    NaiDiffusion3Furry,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ImageSizeDto {
    pub width: u32,
    pub height: u32,
}

impl Default for ImageSizeDto {
    fn default() -> Self {
        Self {
            width: 832,
            height: 1216,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SamplerDto {
    KEuler,
    #[default]
    KEulerAncestral,
    KDpm2,
    KDpm2Ancestral,
    KDpmpp2m,
    KDpmpp2sAncestral,
    KDpmppSde,
    Ddim,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NoiseScheduleDto {
    #[default]
    Karras,
    Exponential,
    Polyexponential,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UcPresetDto {
    Heavy,
    #[default]
    Light,
    FurryFocus,
    HumanFocus,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormatDto {
    Png,
    Webp,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StreamModeDto {
    #[default]
    Sse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct Img2ImgRequestDto {
    pub image: ImageInputDto,
    pub strength: f32,
    pub noise: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<ImageInputDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ControlNetInputDto {
    pub encoding: ResourceRefDto,
    pub info_extracted: f32,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ControlNetConfigDto {
    pub images: Vec<ControlNetInputDto>,
    pub strength: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CharacterReferenceTypeDto {
    Character,
    Style,
    CharacterAndStyle,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct CharacterReferenceDto {
    pub image: ImageInputDto,
    pub reference_type: CharacterReferenceTypeDto,
    pub fidelity: f32,
    pub strength: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct CharacterPositionDto {
    pub x: f32,
    pub y: f32,
}

impl Default for CharacterPositionDto {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct CharacterDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    pub position: CharacterPositionDto,
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GenerationDraftSeedModeDto {
    Random,
    Fixed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GenerationDraftCharacterPositionModeDto {
    Global,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftI2iDto {
    pub image: ResourceRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<ResourceRefDto>,
    pub strength: f32,
    pub noise: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftVibeSlotDto {
    pub id: String,
    pub encoding: ResourceRefDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vibe_id: Option<String>,
    pub information_extracted: f32,
    pub strength: f32,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_image: Option<ResourceRefDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftVibeDto {
    pub enabled: bool,
    pub strength: f32,
    pub slots: Vec<GenerationDraftVibeSlotDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftPreciseReferenceDto {
    pub id: String,
    pub image: ResourceRefDto,
    pub reference_type: CharacterReferenceTypeDto,
    pub fidelity: f32,
    pub strength: f32,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftCharacterDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    pub prompt: String,
    pub negative_prompt: String,
    pub enabled: bool,
    pub position: CharacterPositionDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[allow(clippy::struct_excessive_bools)]
pub struct GenerationDraftDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_preset_id: Option<String>,
    pub prompt: String,
    pub negative_prompt: String,
    pub model: ImageModelDto,
    pub size: ImageSizeDto,
    pub quality: bool,
    pub uc_preset: UcPresetDto,
    pub steps: u32,
    pub scale: f32,
    pub sampler: SamplerDto,
    pub noise_schedule: NoiseScheduleDto,
    pub seed_mode: GenerationDraftSeedModeDto,
    pub seed: i64,
    pub n_samples: u32,
    pub request_count: u32,
    pub cfg_rescale: f32,
    pub variety_boost: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_format: Option<ImageFormatDto>,
    pub strict_mode: bool,
    pub stream_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i2i: Option<GenerationDraftI2iDto>,
    pub vibe: GenerationDraftVibeDto,
    pub precise_references: Vec<GenerationDraftPreciseReferenceDto>,
    pub characters: Vec<GenerationDraftCharacterDto>,
    pub character_position_mode: GenerationDraftCharacterPositionModeDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SaveGenerationDraftRequestDto {
    pub draft: GenerationDraftDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerateImageRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_preset_id: Option<String>,
    pub prompt: String,
    pub model: ImageModelDto,
    pub size: ImageSizeDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
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
    pub strict_mode: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub i2i: Option<Img2ImgRequestDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controlnet: Option<ControlNetConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_references: Option<Vec<CharacterReferenceDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters: Option<Vec<CharacterDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_coords: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_format: Option<ImageFormatDto>,
}

impl Default for GenerateImageRequestDto {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            main_preset_id: None,
            model: ImageModelDto::default(),
            size: ImageSizeDto::default(),
            negative_prompt: None,
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
            strict_mode: false,
            i2i: None,
            controlnet: None,
            character_references: None,
            characters: None,
            use_coords: None,
            image_format: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerateImageStreamRequestDto {
    pub base: GenerateImageRequestDto,
    pub stream: StreamModeDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum GenerationWorkRequestDto {
    Image(GenerateImageRequestDto),
    Stream(GenerateImageStreamRequestDto),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationPlanContextDto {
    pub request_count: u32,
    pub pending_vibe_encode_count: u32,
    pub is_opus: bool,
}

impl Default for GenerationPlanContextDto {
    fn default() -> Self {
        Self {
            request_count: 1,
            pending_vibe_encode_count: 0,
            is_opus: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SubmitGenerationRequestDto {
    pub batch_id: String,
    pub job_id: String,
    pub work: GenerationWorkRequestDto,
    pub context: GenerationPlanContextDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SubmitGenerationBatchJobDto {
    pub job_id: String,
    pub work: GenerationWorkRequestDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SubmitGenerationBatchRequestDto {
    pub batch_id: String,
    pub jobs: Vec<SubmitGenerationBatchJobDto>,
    pub context: GenerationPlanContextDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationEstimateRequestDto {
    pub request: GenerateImageRequestDto,
    pub context: GenerationPlanContextDto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationAnlasEstimateDto {
    pub per_sample_cost: u64,
    pub per_request_cost: u64,
    pub total_cost: u64,
    pub adjusted_resolution: u64,
    pub opus_discount_applied: bool,
    pub pending_encode_cost: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunGenerationJobRequestDto {
    pub job_id: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct QueueDelayDto {
    pub min_ms: u64,
    pub max_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueueDirectiveDto {
    StartJob { job_id: String },
    Wait { delay: QueueDelayDto },
    Paused,
    Idle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationStatusDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_status: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationStatusQueryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}
