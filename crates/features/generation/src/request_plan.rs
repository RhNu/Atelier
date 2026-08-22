use crate::{
    GenerateImageRequest, GenerateImageStreamRequest, GenerationError, StreamMode,
    normalize::resolve_use_coords, normalize_generate_request,
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

/// Account and run state that affects the Anlas price of a planned generation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenerationPlanContext {
    /// Number of identical requests the run submits.
    pub request_count: u32,
    /// Number of Vibe encodings the run still has to perform.
    pub pending_vibe_encode_count: u32,
    /// Numeric subscription tier.
    pub tier: i32,
    /// Whether the paid subscription is currently active.
    pub subscription_active: bool,
    /// Whether the free V5 generation allowance is overdrawn.
    pub v5_usage_is_negative: bool,
}

impl Default for GenerationPlanContext {
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

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationRequestPlan {
    pub normalized_request: GenerateImageRequest,
    pub seed_mode: SeedMode,
    pub output_mode: GenerationOutputMode,
    pub resolved_use_coords: bool,
}

/// Builds a normalized, host-neutral plan for a non-streaming generation request.
///
/// # Errors
/// Returns [`GenerationError`] when request validation or normalization fails.
pub fn plan_generation_request(
    request: GenerateImageRequest,
    _context: GenerationPlanContext,
) -> Result<GenerationRequestPlan, GenerationError> {
    let normalized_request = normalize_generate_request(request)?;
    Ok(build_plan(normalized_request, GenerationOutputMode::Image))
}

/// Builds a normalized, host-neutral plan for a streaming generation request.
///
/// # Errors
/// Returns [`GenerationError`] when base request validation or normalization fails.
pub fn plan_generation_stream_request(
    request: GenerateImageStreamRequest,
    _context: GenerationPlanContext,
) -> Result<GenerationRequestPlan, GenerationError> {
    let normalized_request = normalize_generate_request(request.base)?;
    Ok(build_plan(
        normalized_request,
        GenerationOutputMode::Stream(request.stream),
    ))
}

fn build_plan(
    normalized_request: GenerateImageRequest,
    output_mode: GenerationOutputMode,
) -> GenerationRequestPlan {
    let seed_mode = if normalized_request.seed == 0 {
        SeedMode::Auto
    } else {
        SeedMode::Fixed(normalized_request.seed)
    };
    let resolved_use_coords = resolve_use_coords(&normalized_request);

    GenerationRequestPlan {
        normalized_request,
        seed_mode,
        output_mode,
        resolved_use_coords,
    }
}
