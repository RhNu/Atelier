use crate::{
    AnlasEstimate, AnlasEstimateInput, GenerateImageRequest, GenerateImageStreamRequest,
    GenerationError, StreamMode, estimate_anlas_cost, normalize::resolve_use_coords,
    normalize_generate_request,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SeedMode {
    Auto,
    Fixed(i64),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GenerationOutputMode {
    Image,
    Stream(StreamMode),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenerationPlanContext {
    pub request_count: u32,
    pub pending_vibe_encode_count: u32,
    pub is_opus: bool,
}

impl Default for GenerationPlanContext {
    fn default() -> Self {
        Self {
            request_count: 1,
            pending_vibe_encode_count: 0,
            is_opus: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationRequestPlan {
    pub normalized_request: GenerateImageRequest,
    pub seed_mode: SeedMode,
    pub output_mode: GenerationOutputMode,
    pub resolved_use_coords: bool,
    pub anlas_estimate: AnlasEstimate,
}

/// Builds a normalized, host-neutral plan for a non-streaming generation request.
///
/// # Errors
/// Returns [`GenerationError`] when request validation or normalization fails.
pub fn plan_generation_request(
    request: GenerateImageRequest,
    context: GenerationPlanContext,
) -> Result<GenerationRequestPlan, GenerationError> {
    let normalized_request = normalize_generate_request(request)?;
    Ok(build_plan(
        normalized_request,
        GenerationOutputMode::Image,
        context,
    ))
}

/// Builds a normalized, host-neutral plan for a streaming generation request.
///
/// # Errors
/// Returns [`GenerationError`] when base request validation or normalization fails.
pub fn plan_generation_stream_request(
    request: GenerateImageStreamRequest,
    context: GenerationPlanContext,
) -> Result<GenerationRequestPlan, GenerationError> {
    let normalized_request = normalize_generate_request(request.base)?;
    Ok(build_plan(
        normalized_request,
        GenerationOutputMode::Stream(request.stream),
        context,
    ))
}

fn build_plan(
    normalized_request: GenerateImageRequest,
    output_mode: GenerationOutputMode,
    context: GenerationPlanContext,
) -> GenerationRequestPlan {
    let seed_mode = if normalized_request.seed == 0 {
        SeedMode::Auto
    } else {
        SeedMode::Fixed(normalized_request.seed)
    };
    let resolved_use_coords = resolve_use_coords(&normalized_request);
    let anlas_estimate = estimate_anlas_cost(AnlasEstimateInput {
        width: normalized_request.size.width,
        height: normalized_request.size.height,
        steps: normalized_request.steps,
        n_samples: normalized_request.n_samples,
        request_count: context.request_count,
        has_img2img: normalized_request.img2img.is_some(),
        img2img_strength: normalized_request
            .img2img
            .as_ref()
            .map_or(0.7, |i2i| i2i.strength),
        has_director_reference: normalized_request
            .character_references
            .as_ref()
            .is_some_and(|references| !references.is_empty()),
        pending_encode_count: context.pending_vibe_encode_count,
        is_opus: context.is_opus,
    });

    GenerationRequestPlan {
        normalized_request,
        seed_mode,
        output_mode,
        resolved_use_coords,
        anlas_estimate,
    }
}
