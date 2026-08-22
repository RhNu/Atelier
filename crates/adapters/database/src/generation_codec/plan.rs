use super::{
    DatabaseResult, Deserialize, GenerateImageRequestDto, GenerationOutputMode,
    GenerationPlanContext, GenerationRequestPlan, SeedMode, Serialize, stream_mode_as_str,
    stream_mode_from_str,
};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub(super) struct GenerationPlanContextDto {
    request_count: u32,
    pending_vibe_encode_count: u32,
    tier: i32,
    subscription_active: bool,
    v5_usage_is_negative: bool,
}

impl From<&GenerationPlanContext> for GenerationPlanContextDto {
    fn from(value: &GenerationPlanContext) -> Self {
        Self {
            request_count: value.request_count,
            pending_vibe_encode_count: value.pending_vibe_encode_count,
            tier: value.tier,
            subscription_active: value.subscription_active,
            v5_usage_is_negative: value.v5_usage_is_negative,
        }
    }
}

impl GenerationPlanContextDto {
    pub(super) const fn into_domain(self) -> GenerationPlanContext {
        GenerationPlanContext {
            request_count: self.request_count,
            pending_vibe_encode_count: self.pending_vibe_encode_count,
            tier: self.tier,
            subscription_active: self.subscription_active,
            v5_usage_is_negative: self.v5_usage_is_negative,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct GenerationRequestPlanDto {
    normalized_request: GenerateImageRequestDto,
    seed_mode: SeedModeDto,
    output_mode: GenerationOutputModeDto,
    resolved_use_coords: bool,
}

impl From<&GenerationRequestPlan> for GenerationRequestPlanDto {
    fn from(value: &GenerationRequestPlan) -> Self {
        Self {
            normalized_request: GenerateImageRequestDto::from(&value.normalized_request),
            seed_mode: SeedModeDto::from(value.seed_mode),
            output_mode: GenerationOutputModeDto::from(value.output_mode),
            resolved_use_coords: value.resolved_use_coords,
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
