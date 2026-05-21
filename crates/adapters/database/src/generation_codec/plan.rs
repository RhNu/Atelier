use super::{
    AnlasEstimate, DatabaseResult, Deserialize, GenerateImageRequestDto, GenerationOutputMode,
    GenerationPlanContext, GenerationRequestPlan, SeedMode, Serialize, stream_mode_as_str,
    stream_mode_from_str,
};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub(super) struct GenerationPlanContextDto {
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
    pub(super) const fn into_domain(self) -> GenerationPlanContext {
        GenerationPlanContext {
            request_count: self.request_count,
            pending_vibe_encode_count: self.pending_vibe_encode_count,
            is_opus: self.is_opus,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct GenerationRequestPlanDto {
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
    pub(super) fn into_domain(self) -> DatabaseResult<GenerationRequestPlan> {
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
pub(super) enum SeedModeDto {
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
    pub(super) const fn into_domain(self) -> SeedMode {
        match self {
            Self::Auto => SeedMode::Auto,
            Self::Fixed(seed) => SeedMode::Fixed(seed),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "stream", rename_all = "snake_case")]
pub(super) enum GenerationOutputModeDto {
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
    pub(super) fn into_domain(self) -> DatabaseResult<GenerationOutputMode> {
        match self {
            Self::Image => Ok(GenerationOutputMode::Image),
            Self::Stream(stream) => {
                Ok(GenerationOutputMode::Stream(stream_mode_from_str(&stream)?))
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub(super) struct AnlasEstimateDto {
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
    pub(super) const fn into_domain(self) -> AnlasEstimate {
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
