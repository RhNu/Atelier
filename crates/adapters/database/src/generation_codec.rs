#![allow(clippy::missing_const_for_fn)]

use nai_atelier_generation::{
    AnlasEstimate, Character, CharacterPosition, CharacterReference, CharacterReferenceType,
    ControlNetConfig, ControlNetInput, GenerateImageRequest, GenerateImageStreamRequest,
    GenerationOutputMode, GenerationPlanContext, GenerationRequestPlan, ImageFormat, ImageModel,
    ImageSize, Img2ImgRequest, NoiseSchedule, Sampler, SeedMode, StreamMode, UcPreset,
};
use nai_atelier_jobs::{BatchId, JobId, JobPayloadRef};
use nai_atelier_kernel::{
    GenerationWorkRequest, PreparedGenerationPayload, SubmittedGenerationPayload,
};
use nai_atelier_prompt_resources::{CompiledPrompt, PromptFunctionTraceEntry, PromptTrace};
use serde::{Deserialize, Serialize};

use crate::codec::{JsonCodec, decode_error};
use crate::error::{DatabaseError, DatabaseResult};

const JSON_SCHEMA_VERSION: u32 = 1;

fn ensure_schema(version: u32) -> DatabaseResult<()> {
    if version == JSON_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(DatabaseError::new(format!(
            "unsupported JSON schema version {version}"
        )))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmittedGenerationPayloadDto {
    schema_version: u32,
    payload_ref: String,
    batch_id: String,
    job_id: String,
    request: GenerationWorkRequestDto,
    context: GenerationPlanContextDto,
}

impl JsonCodec<SubmittedGenerationPayload> for SubmittedGenerationPayloadDto {
    fn from_domain(value: &SubmittedGenerationPayload) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            payload_ref: value.payload_ref.as_str().to_owned(),
            batch_id: value.batch_id.as_str().to_owned(),
            job_id: value.job_id.as_str().to_owned(),
            request: GenerationWorkRequestDto::from(&value.request),
            context: GenerationPlanContextDto::from(&value.context),
        }
    }

    fn into_domain(self) -> DatabaseResult<SubmittedGenerationPayload> {
        ensure_schema(self.schema_version)?;
        Ok(SubmittedGenerationPayload {
            payload_ref: JobPayloadRef::new(self.payload_ref),
            batch_id: BatchId::new(self.batch_id),
            job_id: JobId::new(self.job_id),
            request: self.request.into_domain()?,
            context: self.context.into_domain(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreparedGenerationPayloadDto {
    schema_version: u32,
    payload_ref: String,
    submitted_payload_ref: String,
    batch_id: String,
    job_id: String,
    request: GenerationWorkRequestDto,
    compiled_prompt: CompiledPromptDto,
    plan: GenerationRequestPlanDto,
}

impl JsonCodec<PreparedGenerationPayload> for PreparedGenerationPayloadDto {
    fn from_domain(value: &PreparedGenerationPayload) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            payload_ref: value.payload_ref.as_str().to_owned(),
            submitted_payload_ref: value.submitted_payload_ref.as_str().to_owned(),
            batch_id: value.batch_id.as_str().to_owned(),
            job_id: value.job_id.as_str().to_owned(),
            request: GenerationWorkRequestDto::from(&value.request),
            compiled_prompt: CompiledPromptDto::from(&value.compiled_prompt),
            plan: GenerationRequestPlanDto::from(&value.plan),
        }
    }

    fn into_domain(self) -> DatabaseResult<PreparedGenerationPayload> {
        ensure_schema(self.schema_version)?;
        Ok(PreparedGenerationPayload {
            payload_ref: JobPayloadRef::new(self.payload_ref),
            submitted_payload_ref: JobPayloadRef::new(self.submitted_payload_ref),
            batch_id: BatchId::new(self.batch_id),
            job_id: JobId::new(self.job_id),
            request: self.request.into_domain()?,
            compiled_prompt: self.compiled_prompt.into_domain(),
            plan: self.plan.into_domain()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
enum GenerationWorkRequestDto {
    Image(GenerateImageRequestDto),
    Stream(GenerateImageStreamRequestDto),
}

impl From<&GenerationWorkRequest> for GenerationWorkRequestDto {
    fn from(value: &GenerationWorkRequest) -> Self {
        match value {
            GenerationWorkRequest::Image(request) => {
                Self::Image(GenerateImageRequestDto::from(request))
            }
            GenerationWorkRequest::Stream(request) => {
                Self::Stream(GenerateImageStreamRequestDto::from(request))
            }
        }
    }
}

impl GenerationWorkRequestDto {
    fn into_domain(self) -> DatabaseResult<GenerationWorkRequest> {
        match self {
            Self::Image(request) => Ok(GenerationWorkRequest::Image(request.into_domain()?)),
            Self::Stream(request) => Ok(GenerationWorkRequest::Stream(request.into_domain()?)),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerateImageStreamRequestDto {
    base: GenerateImageRequestDto,
    stream: String,
}

impl From<&GenerateImageStreamRequest> for GenerateImageStreamRequestDto {
    fn from(value: &GenerateImageStreamRequest) -> Self {
        Self {
            base: GenerateImageRequestDto::from(&value.base),
            stream: stream_mode_as_str(value.stream).to_owned(),
        }
    }
}

impl GenerateImageStreamRequestDto {
    fn into_domain(self) -> DatabaseResult<GenerateImageStreamRequest> {
        Ok(GenerateImageStreamRequest {
            base: self.base.into_domain()?,
            stream: stream_mode_from_str(&self.stream)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerateImageRequestDto {
    prompt: String,
    model: String,
    width: u32,
    height: u32,
    negative_prompt: Option<String>,
    quality: bool,
    uc_preset: String,
    steps: u32,
    scale: f32,
    sampler: String,
    noise_schedule: String,
    seed: i64,
    n_samples: u32,
    cfg_rescale: f32,
    variety_boost: bool,
    i2i: Option<Img2ImgRequestDto>,
    controlnet: Option<ControlNetConfigDto>,
    character_references: Option<Vec<CharacterReferenceDto>>,
    characters: Option<Vec<CharacterDto>>,
    use_coords: Option<bool>,
    image_format: Option<String>,
    strict_mode: bool,
}

impl From<&GenerateImageRequest> for GenerateImageRequestDto {
    fn from(value: &GenerateImageRequest) -> Self {
        Self {
            prompt: value.prompt.clone(),
            model: image_model_as_str(value.model).to_owned(),
            width: value.size.width,
            height: value.size.height,
            negative_prompt: value.negative_prompt.clone(),
            quality: value.quality,
            uc_preset: uc_preset_as_str(value.uc_preset).to_owned(),
            steps: value.steps,
            scale: value.scale,
            sampler: sampler_as_str(value.sampler).to_owned(),
            noise_schedule: noise_schedule_as_str(value.noise_schedule).to_owned(),
            seed: value.seed,
            n_samples: value.n_samples,
            cfg_rescale: value.cfg_rescale,
            variety_boost: value.variety_boost,
            i2i: value.i2i.as_ref().map(Img2ImgRequestDto::from),
            controlnet: value.controlnet.as_ref().map(ControlNetConfigDto::from),
            character_references: value
                .character_references
                .as_ref()
                .map(|items| items.iter().map(CharacterReferenceDto::from).collect()),
            characters: value
                .characters
                .as_ref()
                .map(|items| items.iter().map(CharacterDto::from).collect()),
            use_coords: value.use_coords,
            image_format: value
                .image_format
                .map(image_format_as_str)
                .map(str::to_owned),
            strict_mode: value.strict_mode,
        }
    }
}

impl GenerateImageRequestDto {
    fn into_domain(self) -> DatabaseResult<GenerateImageRequest> {
        Ok(GenerateImageRequest {
            prompt: self.prompt,
            model: image_model_from_str(&self.model)?,
            size: ImageSize {
                width: self.width,
                height: self.height,
            },
            negative_prompt: self.negative_prompt,
            quality: self.quality,
            uc_preset: uc_preset_from_str(&self.uc_preset)?,
            steps: self.steps,
            scale: self.scale,
            sampler: sampler_from_str(&self.sampler)?,
            noise_schedule: noise_schedule_from_str(&self.noise_schedule)?,
            seed: self.seed,
            n_samples: self.n_samples,
            cfg_rescale: self.cfg_rescale,
            variety_boost: self.variety_boost,
            i2i: self.i2i.map(Into::into),
            controlnet: self.controlnet.map(Into::into),
            character_references: self
                .character_references
                .map(|items| {
                    items
                        .into_iter()
                        .map(CharacterReferenceDto::into_domain)
                        .collect::<DatabaseResult<Vec<_>>>()
                })
                .transpose()?,
            characters: self
                .characters
                .map(|items| items.into_iter().map(Into::into).collect()),
            use_coords: self.use_coords,
            image_format: self
                .image_format
                .as_deref()
                .map(image_format_from_str)
                .transpose()?,
            strict_mode: self.strict_mode,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Img2ImgRequestDto {
    image: String,
    strength: f32,
    noise: f32,
    mask: Option<String>,
}

impl From<&Img2ImgRequest> for Img2ImgRequestDto {
    fn from(value: &Img2ImgRequest) -> Self {
        Self {
            image: value.image.clone(),
            strength: value.strength,
            noise: value.noise,
            mask: value.mask.clone(),
        }
    }
}

impl From<Img2ImgRequestDto> for Img2ImgRequest {
    fn from(value: Img2ImgRequestDto) -> Self {
        Self {
            image: value.image,
            strength: value.strength,
            noise: value.noise,
            mask: value.mask,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ControlNetInputDto {
    vibe_data_cache: String,
    info_extracted: f32,
    strength: f32,
}

impl From<&ControlNetInput> for ControlNetInputDto {
    fn from(value: &ControlNetInput) -> Self {
        Self {
            vibe_data_cache: value.vibe_data_cache.clone(),
            info_extracted: value.info_extracted,
            strength: value.strength,
        }
    }
}

impl From<ControlNetInputDto> for ControlNetInput {
    fn from(value: ControlNetInputDto) -> Self {
        Self {
            vibe_data_cache: value.vibe_data_cache,
            info_extracted: value.info_extracted,
            strength: value.strength,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ControlNetConfigDto {
    images: Vec<ControlNetInputDto>,
    strength: f32,
}

impl From<&ControlNetConfig> for ControlNetConfigDto {
    fn from(value: &ControlNetConfig) -> Self {
        Self {
            images: value.images.iter().map(ControlNetInputDto::from).collect(),
            strength: value.strength,
        }
    }
}

impl From<ControlNetConfigDto> for ControlNetConfig {
    fn from(value: ControlNetConfigDto) -> Self {
        Self {
            images: value.images.into_iter().map(Into::into).collect(),
            strength: value.strength,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CharacterReferenceDto {
    image: String,
    reference_type: String,
    fidelity: f32,
    strength: f32,
}

impl From<&CharacterReference> for CharacterReferenceDto {
    fn from(value: &CharacterReference) -> Self {
        Self {
            image: value.image.clone(),
            reference_type: character_reference_type_as_str(value.reference_type).to_owned(),
            fidelity: value.fidelity,
            strength: value.strength,
        }
    }
}

impl CharacterReferenceDto {
    fn into_domain(self) -> DatabaseResult<CharacterReference> {
        Ok(CharacterReference {
            image: self.image,
            reference_type: character_reference_type_from_str(&self.reference_type)?,
            fidelity: self.fidelity,
            strength: self.strength,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CharacterDto {
    prompt: String,
    negative_prompt: Option<String>,
    x: f32,
    y: f32,
    enabled: bool,
}

impl From<&Character> for CharacterDto {
    fn from(value: &Character) -> Self {
        Self {
            prompt: value.prompt.clone(),
            negative_prompt: value.negative_prompt.clone(),
            x: value.position.x,
            y: value.position.y,
            enabled: value.enabled,
        }
    }
}

impl From<CharacterDto> for Character {
    fn from(value: CharacterDto) -> Self {
        Self {
            prompt: value.prompt,
            negative_prompt: value.negative_prompt,
            position: CharacterPosition {
                x: value.x,
                y: value.y,
            },
            enabled: value.enabled,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct GenerationPlanContextDto {
    request_count: u32,
    pending_vibe_encode_count: u32,
    is_opus: bool,
}

impl From<&GenerationPlanContext> for GenerationPlanContextDto {
    fn from(value: &GenerationPlanContext) -> Self {
        Self {
            request_count: value.request_count,
            pending_vibe_encode_count: value.pending_vibe_encode_count,
            is_opus: value.is_opus,
        }
    }
}

impl GenerationPlanContextDto {
    const fn into_domain(self) -> GenerationPlanContext {
        GenerationPlanContext {
            request_count: self.request_count,
            pending_vibe_encode_count: self.pending_vibe_encode_count,
            is_opus: self.is_opus,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationRequestPlanDto {
    normalized_request: GenerateImageRequestDto,
    seed_mode: SeedModeDto,
    output_mode: GenerationOutputModeDto,
    resolved_use_coords: bool,
    anlas_estimate: AnlasEstimateDto,
}

impl From<&GenerationRequestPlan> for GenerationRequestPlanDto {
    fn from(value: &GenerationRequestPlan) -> Self {
        Self {
            normalized_request: GenerateImageRequestDto::from(&value.normalized_request),
            seed_mode: SeedModeDto::from(value.seed_mode),
            output_mode: GenerationOutputModeDto::from(value.output_mode),
            resolved_use_coords: value.resolved_use_coords,
            anlas_estimate: AnlasEstimateDto::from(value.anlas_estimate),
        }
    }
}

impl GenerationRequestPlanDto {
    fn into_domain(self) -> DatabaseResult<GenerationRequestPlan> {
        Ok(GenerationRequestPlan {
            normalized_request: self.normalized_request.into_domain()?,
            seed_mode: self.seed_mode.into_domain(),
            output_mode: self.output_mode.into_domain()?,
            resolved_use_coords: self.resolved_use_coords,
            anlas_estimate: self.anlas_estimate.into_domain(),
        })
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum SeedModeDto {
    Auto,
    Fixed(i64),
}

impl From<SeedMode> for SeedModeDto {
    fn from(value: SeedMode) -> Self {
        match value {
            SeedMode::Auto => Self::Auto,
            SeedMode::Fixed(seed) => Self::Fixed(seed),
        }
    }
}

impl SeedModeDto {
    const fn into_domain(self) -> SeedMode {
        match self {
            Self::Auto => SeedMode::Auto,
            Self::Fixed(seed) => SeedMode::Fixed(seed),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "stream", rename_all = "snake_case")]
enum GenerationOutputModeDto {
    Image,
    Stream(String),
}

impl From<GenerationOutputMode> for GenerationOutputModeDto {
    fn from(value: GenerationOutputMode) -> Self {
        match value {
            GenerationOutputMode::Image => Self::Image,
            GenerationOutputMode::Stream(stream) => {
                Self::Stream(stream_mode_as_str(stream).to_owned())
            }
        }
    }
}

impl GenerationOutputModeDto {
    fn into_domain(self) -> DatabaseResult<GenerationOutputMode> {
        match self {
            Self::Image => Ok(GenerationOutputMode::Image),
            Self::Stream(stream) => {
                Ok(GenerationOutputMode::Stream(stream_mode_from_str(&stream)?))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct AnlasEstimateDto {
    per_sample_cost: u64,
    per_request_cost: u64,
    total_cost: u64,
    adjusted_resolution: u64,
    opus_discount_applied: bool,
    pending_encode_cost: u64,
}

impl From<AnlasEstimate> for AnlasEstimateDto {
    fn from(value: AnlasEstimate) -> Self {
        Self {
            per_sample_cost: value.per_sample_cost,
            per_request_cost: value.per_request_cost,
            total_cost: value.total_cost,
            adjusted_resolution: value.adjusted_resolution,
            opus_discount_applied: value.opus_discount_applied,
            pending_encode_cost: value.pending_encode_cost,
        }
    }
}

impl AnlasEstimateDto {
    const fn into_domain(self) -> AnlasEstimate {
        AnlasEstimate {
            per_sample_cost: self.per_sample_cost,
            per_request_cost: self.per_request_cost,
            total_cost: self.total_cost,
            adjusted_resolution: self.adjusted_resolution,
            opus_discount_applied: self.opus_discount_applied,
            pending_encode_cost: self.pending_encode_cost,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CompiledPromptDto {
    expanded_prompt: String,
    trace: PromptTraceDto,
}

impl From<&CompiledPrompt> for CompiledPromptDto {
    fn from(value: &CompiledPrompt) -> Self {
        Self {
            expanded_prompt: value.expanded_prompt.clone(),
            trace: PromptTraceDto::from(&value.trace),
        }
    }
}

impl CompiledPromptDto {
    fn into_domain(self) -> CompiledPrompt {
        CompiledPrompt {
            expanded_prompt: self.expanded_prompt,
            trace: self.trace.into_domain(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PromptTraceDto {
    raw_prompt: String,
    expanded_prompt: String,
    function_calls: Vec<PromptFunctionTraceEntryDto>,
}

impl From<&PromptTrace> for PromptTraceDto {
    fn from(value: &PromptTrace) -> Self {
        Self {
            raw_prompt: value.raw_prompt.clone(),
            expanded_prompt: value.expanded_prompt.clone(),
            function_calls: value
                .function_calls
                .iter()
                .map(PromptFunctionTraceEntryDto::from)
                .collect(),
        }
    }
}

impl PromptTraceDto {
    fn into_domain(self) -> PromptTrace {
        PromptTrace {
            raw_prompt: self.raw_prompt,
            expanded_prompt: self.expanded_prompt,
            function_calls: self
                .function_calls
                .into_iter()
                .map(PromptFunctionTraceEntryDto::into_domain)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PromptFunctionTraceEntryDto {
    function_name: String,
    raw_call: String,
    resolved_arguments: Vec<String>,
    result_text: Option<String>,
    depth: usize,
    call_chain: Vec<String>,
}

impl From<&PromptFunctionTraceEntry> for PromptFunctionTraceEntryDto {
    fn from(value: &PromptFunctionTraceEntry) -> Self {
        Self {
            function_name: value.function_name.clone(),
            raw_call: value.raw_call.clone(),
            resolved_arguments: value.resolved_arguments.clone(),
            result_text: value.result_text.clone(),
            depth: value.depth,
            call_chain: value.call_chain.clone(),
        }
    }
}

impl PromptFunctionTraceEntryDto {
    fn into_domain(self) -> PromptFunctionTraceEntry {
        PromptFunctionTraceEntry {
            function_name: self.function_name,
            raw_call: self.raw_call,
            resolved_arguments: self.resolved_arguments,
            result_text: self.result_text,
            depth: self.depth,
            call_chain: self.call_chain,
        }
    }
}

const fn image_model_as_str(value: ImageModel) -> &'static str {
    value.as_str()
}

fn image_model_from_str(value: &str) -> DatabaseResult<ImageModel> {
    match value {
        "nai-diffusion-4-5-full" => Ok(ImageModel::NaiDiffusion45Full),
        "nai-diffusion-4-5-curated" => Ok(ImageModel::NaiDiffusion45Curated),
        "nai-diffusion-4-full" => Ok(ImageModel::NaiDiffusion4Full),
        "nai-diffusion-4-curated" => Ok(ImageModel::NaiDiffusion4Curated),
        "nai-diffusion-3" => Ok(ImageModel::NaiDiffusion3),
        "nai-diffusion-3-furry" => Ok(ImageModel::NaiDiffusion3Furry),
        _ => Err(decode_error("image model", value)),
    }
}

const fn sampler_as_str(value: Sampler) -> &'static str {
    match value {
        Sampler::KEuler => "k_euler",
        Sampler::KEulerAncestral => "k_euler_ancestral",
        Sampler::KDpm2 => "k_dpm2",
        Sampler::KDpm2Ancestral => "k_dpm2_ancestral",
        Sampler::KDpmpp2m => "k_dpmpp_2m",
        Sampler::KDpmpp2sAncestral => "k_dpmpp_2s_ancestral",
        Sampler::KDpmppSde => "k_dpmpp_sde",
        Sampler::Ddim => "ddim",
    }
}

fn sampler_from_str(value: &str) -> DatabaseResult<Sampler> {
    match value {
        "k_euler" => Ok(Sampler::KEuler),
        "k_euler_ancestral" => Ok(Sampler::KEulerAncestral),
        "k_dpm2" => Ok(Sampler::KDpm2),
        "k_dpm2_ancestral" => Ok(Sampler::KDpm2Ancestral),
        "k_dpmpp_2m" => Ok(Sampler::KDpmpp2m),
        "k_dpmpp_2s_ancestral" => Ok(Sampler::KDpmpp2sAncestral),
        "k_dpmpp_sde" => Ok(Sampler::KDpmppSde),
        "ddim" => Ok(Sampler::Ddim),
        _ => Err(decode_error("sampler", value)),
    }
}

const fn noise_schedule_as_str(value: NoiseSchedule) -> &'static str {
    match value {
        NoiseSchedule::Karras => "karras",
        NoiseSchedule::Exponential => "exponential",
        NoiseSchedule::Polyexponential => "polyexponential",
    }
}

fn noise_schedule_from_str(value: &str) -> DatabaseResult<NoiseSchedule> {
    match value {
        "karras" => Ok(NoiseSchedule::Karras),
        "exponential" => Ok(NoiseSchedule::Exponential),
        "polyexponential" => Ok(NoiseSchedule::Polyexponential),
        _ => Err(decode_error("noise schedule", value)),
    }
}

const fn uc_preset_as_str(value: UcPreset) -> &'static str {
    match value {
        UcPreset::Heavy => "heavy",
        UcPreset::Light => "light",
        UcPreset::FurryFocus => "furry_focus",
        UcPreset::HumanFocus => "human_focus",
        UcPreset::None => "none",
    }
}

fn uc_preset_from_str(value: &str) -> DatabaseResult<UcPreset> {
    match value {
        "heavy" => Ok(UcPreset::Heavy),
        "light" => Ok(UcPreset::Light),
        "furry_focus" => Ok(UcPreset::FurryFocus),
        "human_focus" => Ok(UcPreset::HumanFocus),
        "none" => Ok(UcPreset::None),
        _ => Err(decode_error("uc preset", value)),
    }
}

const fn image_format_as_str(value: ImageFormat) -> &'static str {
    match value {
        ImageFormat::Png => "png",
        ImageFormat::Webp => "webp",
    }
}

fn image_format_from_str(value: &str) -> DatabaseResult<ImageFormat> {
    match value {
        "png" => Ok(ImageFormat::Png),
        "webp" => Ok(ImageFormat::Webp),
        _ => Err(decode_error("image format", value)),
    }
}

const fn stream_mode_as_str(value: StreamMode) -> &'static str {
    match value {
        StreamMode::Sse => "sse",
    }
}

fn stream_mode_from_str(value: &str) -> DatabaseResult<StreamMode> {
    match value {
        "sse" => Ok(StreamMode::Sse),
        _ => Err(decode_error("stream mode", value)),
    }
}

const fn character_reference_type_as_str(value: CharacterReferenceType) -> &'static str {
    match value {
        CharacterReferenceType::Character => "character",
        CharacterReferenceType::Style => "style",
        CharacterReferenceType::CharacterAndStyle => "character_and_style",
    }
}

fn character_reference_type_from_str(value: &str) -> DatabaseResult<CharacterReferenceType> {
    match value {
        "character" => Ok(CharacterReferenceType::Character),
        "style" => Ok(CharacterReferenceType::Style),
        "character_and_style" => Ok(CharacterReferenceType::CharacterAndStyle),
        _ => Err(decode_error("character reference type", value)),
    }
}
