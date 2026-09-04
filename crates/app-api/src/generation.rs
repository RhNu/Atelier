use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::prompt::CompileGenerationPromptRequestDto;
use crate::resource::{ImageInputDto, ResourceRefDto};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum ImageModelDto {
    #[serde(rename = "nai-diffusion-5-full")]
    NaiDiffusion5Full,
    #[serde(rename = "nai-diffusion-5-curated")]
    NaiDiffusion5Curated,
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
    #[serde(rename = "nai-diffusion-furry-3")]
    NaiDiffusion3Furry,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PromptStructureDto {
    Legacy,
    V4,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CharacterPositionModeDto {
    #[serde(rename = "grid_5x5")]
    Grid5x5,
    Freeform,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ModelCapabilitiesDto {
    pub prompt_structure: PromptStructureDto,
    pub params_version: u32,
    pub default_steps: u32,
    pub default_scale: f32,
    pub max_characters: u32,
    pub character_position_mode: Option<CharacterPositionModeDto>,
    pub can_position_one_character: bool,
    pub supports_vibe_transfer: bool,
    pub supports_encoded_vibe: bool,
    pub supports_character_reference: bool,
    pub supports_character_reference_inpainting: bool,
    pub supports_variety_boost: bool,
    pub supports_inpainting: bool,
    pub supports_furry_mode: bool,
    pub supports_streaming: bool,
    pub supports_smea: bool,
    pub supports_dynamic_thresholding: bool,
    pub uses_v5_extensions: bool,
    /// Whether the model's Opus free-first-image allowance is metered by the account's
    /// `v5_usage` pool. Gate Opus generation-allowance UI on this, not on `uses_v5_extensions`.
    pub has_opus_usage_limit: bool,
    pub supports_light_quality_preset: bool,
    pub supports_transparent_background: bool,
    pub variety_sigma_coefficient: Option<f32>,
    pub prompt_token_limit: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct ImageModelDescriptorDto {
    pub model: ImageModelDto,
    pub capabilities: ModelCapabilitiesDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CountPromptTokensRequestDto {
    pub compile: CompileGenerationPromptRequestDto,
    pub quality: QualityPresetDto,
    pub transparent_background: bool,
    pub uc_preset: UcPresetDto,
    #[serde(default)]
    pub furry_mode: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptTokenCountDto {
    pub used: u32,
    pub limit: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterPromptTokenUsageDto {
    pub index: usize,
    pub prompt: PromptTokenCountDto,
    pub negative_prompt: PromptTokenCountDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PromptTokenUsageDto {
    pub prompt: PromptTokenCountDto,
    pub negative_prompt: PromptTokenCountDto,
    pub characters: Vec<CharacterPromptTokenUsageDto>,
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
    KDpmpp2mSde,
    KDpmpp2sAncestral,
    KDpmppSde,
    Ddim,
    DdimV3,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NoiseScheduleDto {
    Native,
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
    pub inpaint: Option<InpaintRequestDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct InpaintRequestDto {
    pub region_to_replace: ImageInputDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct VibeReferenceDto {
    pub encoding: ResourceRefDto,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct VibeTransferConfigDto {
    pub references: Vec<VibeReferenceDto>,
    pub strength: f32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum QualityPresetDto {
    #[default]
    Standard,
    Light,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CharacterReferenceTypeDto {
    Character,
    Style,
    CharacterAndStyle,
    Costume,
    Delta,
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
    pub inpaint: Option<GenerationDraftInpaintSessionDto>,
    pub strength: f32,
    pub noise: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GenerationDraftMaskPatternDto {
    Solid,
    Stripes,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftMaskDisplayDto {
    pub color: String,
    pub opacity: f32,
    pub pattern: GenerationDraftMaskPatternDto,
    pub show_border: bool,
    pub brush_size: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftInpaintSessionDto {
    pub region_to_replace: ResourceRefDto,
    pub display: GenerationDraftMaskDisplayDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<GenerationDraftFocusRegionDto>,
    pub reference_insets: Vec<GenerationDraftReferenceInsetDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftFocusRegionDto {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub minimum_context_area: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerationDraftReferenceInsetDto {
    pub id: String,
    pub image: ResourceRefDto,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub border_enabled: bool,
    pub border_width: u32,
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
    pub model: ImageModelDto,
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
pub struct GenerationDraftPromptStateDto {
    pub model: ImageModelDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_preset_id: Option<String>,
    pub prompt: String,
    pub negative_prompt: String,
    #[serde(default)]
    pub furry_mode: bool,
    pub characters: Vec<GenerationDraftCharacterDto>,
    pub character_position_mode: GenerationDraftCharacterPositionModeDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[allow(clippy::struct_excessive_bools)]
pub struct GenerationDraftDto {
    pub model: ImageModelDto,
    pub prompt_states: Vec<GenerationDraftPromptStateDto>,
    pub size: ImageSizeDto,
    pub quality: QualityPresetDto,
    pub transparent_background: bool,
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
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct SaveGenerationDraftRequestDto {
    pub draft: GenerationDraftDto,
}

#[allow(
    clippy::struct_excessive_bools,
    reason = "generation feature flags are independent NovelAI request controls"
)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct GenerateImageRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_preset_id: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub furry_mode: bool,
    pub model: ImageModelDto,
    pub size: ImageSizeDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    pub quality: QualityPresetDto,
    pub transparent_background: bool,
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
    pub img2img: Option<Img2ImgRequestDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vibe_transfer: Option<VibeTransferConfigDto>,
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
            furry_mode: false,
            main_preset_id: None,
            model: ImageModelDto::default(),
            size: ImageSizeDto::default(),
            negative_prompt: None,
            quality: QualityPresetDto::Standard,
            transparent_background: false,
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
            img2img: None,
            vibe_transfer: None,
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
    pub tier: i32,
    pub subscription_active: bool,
    pub v5_usage_is_negative: bool,
}

impl Default for GenerationPlanContextDto {
    fn default() -> Self {
        Self {
            request_count: 1,
            pending_vibe_encode_count: 0,
            tier: 0,
            subscription_active: false,
            v5_usage_is_negative: false,
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AnlasEstimateStatusDto {
    #[default]
    Available,
    TooExpensive,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationAnlasEstimateDto {
    pub status: AnlasEstimateStatusDto,
    pub per_image_cost: u64,
    pub per_request_cost: u64,
    pub request_count: u32,
    pub generation_cost: u64,
    pub character_reference_cost: u64,
    pub vibe_reference_overage_cost: u64,
    pub pending_encode_cost: u64,
    pub total_cost: u64,
    pub requested_samples: u32,
    pub sample_limit: u32,
    pub priced_samples: u32,
    pub billable_samples: u32,
    pub free_first_image_applied: bool,
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
    pub batch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_status: Option<String>,
    pub requests: Vec<GenerationRequestStatusDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationRequestStatusDto {
    pub job_id: String,
    pub request_index: u32,
    pub expected_samples: u32,
    pub status: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationStatusQueryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}
