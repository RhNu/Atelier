use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoiseScheduleDto {
    #[default]
    Karras,
    Exponential,
    Polyexponential,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UcPresetDto {
    Heavy,
    #[default]
    Light,
    FurryFocus,
    HumanFocus,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormatDto {
    Png,
    Webp,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamModeDto {
    #[default]
    Sse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GenerateImageRequestDto {
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
    pub image_format: Option<ImageFormatDto>,
}

impl Default for GenerateImageRequestDto {
    fn default() -> Self {
        Self {
            prompt: String::new(),
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
            image_format: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GenerateImageStreamRequestDto {
    pub base: GenerateImageRequestDto,
    pub stream: StreamModeDto,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum GenerationWorkRequestDto {
    Image(GenerateImageRequestDto),
    Stream(GenerateImageStreamRequestDto),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SubmitGenerationRequestDto {
    pub batch_id: String,
    pub job_id: String,
    pub work: GenerationWorkRequestDto,
    pub context: GenerationPlanContextDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunGenerationJobRequestDto {
    pub job_id: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueDelayDto {
    pub min_ms: u64,
    pub max_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueueDirectiveDto {
    StartJob { job_id: String },
    Wait { delay: QueueDelayDto },
    Paused,
    Idle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationStatusDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_status: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationStatusQueryDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}
