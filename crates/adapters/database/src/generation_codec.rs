#![allow(clippy::missing_const_for_fn)]

use atelier_generation::{
    AnlasEstimate, Character, CharacterPosition, CharacterReference, CharacterReferenceType,
    ControlNetConfig, ControlNetInput, GenerateImageRequest, GenerateImageStreamRequest,
    GenerationOutputMode, GenerationPlanContext, GenerationRequestPlan, ImageFormat, ImageModel,
    ImageSize, Img2ImgRequest, NoiseSchedule, Sampler, SeedMode, StreamMode, UcPreset,
};
use atelier_jobs::{BatchId, JobId, JobPayloadRef};
use atelier_kernel::{
    GenerationWorkRequest, PreparedGenerationPayload, SubmittedGenerationPayload,
};
use atelier_prompt_resources::{CompiledPrompt, PromptFunctionTraceEntry, PromptTrace};
use serde::{Deserialize, Serialize};

use crate::codec::{JsonCodec, decode_error};
use crate::error::{DatabaseError, DatabaseResult};

mod payload;
mod plan;
mod prompt;
pub mod scalars;
mod work;

pub use payload::{PreparedGenerationPayloadDto, SubmittedGenerationPayloadDto};
use plan::{GenerationPlanContextDto, GenerationRequestPlanDto};
use prompt::CompiledPromptDto;
use scalars::{
    character_reference_type_as_str, character_reference_type_from_str, image_format_as_str,
    image_format_from_str, image_model_as_str, image_model_from_str, noise_schedule_as_str,
    noise_schedule_from_str, sampler_as_str, sampler_from_str, stream_mode_as_str,
    stream_mode_from_str, uc_preset_as_str, uc_preset_from_str,
};
use work::{GenerateImageRequestDto, GenerationWorkRequestDto};

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
