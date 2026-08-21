pub const DEFAULT_STEPS: u32 = 23;
pub const DEFAULT_SCALE: f32 = 5.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PromptStructure {
    Legacy,
    V4,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModelCapabilities {
    pub prompt_structure: PromptStructure,
    pub params_version: u32,
    pub default_steps: u32,
    pub default_scale: f32,
    pub max_characters: u32,
    pub supports_vibe_transfer: bool,
    pub supports_encoded_vibe: bool,
    pub supports_character_reference: bool,
    pub supports_variety_boost: bool,
    pub supports_inpainting: bool,
    pub supports_smea: bool,
    pub supports_dynamic_thresholding: bool,
    pub uses_v5_extensions: bool,
    pub supports_light_quality_preset: bool,
    pub supports_transparent_background: bool,
    pub variety_sigma_coefficient: Option<f32>,
    pub prompt_token_limit: u32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ImageModel {
    NaiDiffusion5Full,
    NaiDiffusion5Curated,
    #[default]
    NaiDiffusion45Full,
    NaiDiffusion45Curated,
    NaiDiffusion4Full,
    NaiDiffusion4Curated,
    NaiDiffusion3,
    NaiDiffusion3Furry,
}

impl ImageModel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NaiDiffusion5Full => "nai-diffusion-5-full",
            Self::NaiDiffusion5Curated => "nai-diffusion-5-curated",
            Self::NaiDiffusion45Full => "nai-diffusion-4-5-full",
            Self::NaiDiffusion45Curated => "nai-diffusion-4-5-curated",
            Self::NaiDiffusion4Full => "nai-diffusion-4-full",
            Self::NaiDiffusion4Curated => "nai-diffusion-4-curated",
            Self::NaiDiffusion3 => "nai-diffusion-3",
            Self::NaiDiffusion3Furry => "nai-diffusion-furry-3",
        }
    }

    #[must_use]
    pub const fn is_v4(self) -> bool {
        matches!(
            self,
            Self::NaiDiffusion45Full
                | Self::NaiDiffusion45Curated
                | Self::NaiDiffusion4Full
                | Self::NaiDiffusion4Curated
        )
    }

    #[must_use]
    pub const fn is_v5(self) -> bool {
        matches!(self, Self::NaiDiffusion5Full | Self::NaiDiffusion5Curated)
    }

    #[must_use]
    pub const fn is_v45(self) -> bool {
        matches!(self, Self::NaiDiffusion45Full | Self::NaiDiffusion45Curated)
    }

    #[must_use]
    pub const fn vibe_model_key(self) -> Option<&'static str> {
        match self {
            Self::NaiDiffusion5Full | Self::NaiDiffusion5Curated => None,
            Self::NaiDiffusion45Full => Some("v4-5full"),
            Self::NaiDiffusion45Curated => Some("v4-5curated"),
            Self::NaiDiffusion4Full => Some("v4full"),
            Self::NaiDiffusion4Curated => Some("v4curated"),
            Self::NaiDiffusion3 => Some("v3"),
            Self::NaiDiffusion3Furry => Some("v3furry"),
        }
    }

    #[must_use]
    pub fn from_vibe_model_key(value: &str) -> Option<Self> {
        match value {
            "v4-5full" => Some(Self::NaiDiffusion45Full),
            "v4-5curated" => Some(Self::NaiDiffusion45Curated),
            "v4full" => Some(Self::NaiDiffusion4Full),
            "v4curated" => Some(Self::NaiDiffusion4Curated),
            "v3" => Some(Self::NaiDiffusion3),
            "v3furry" => Some(Self::NaiDiffusion3Furry),
            _ => None,
        }
    }

    #[must_use]
    pub const fn capabilities(self) -> ModelCapabilities {
        match self {
            Self::NaiDiffusion5Full => ModelCapabilities {
                prompt_token_limit: 1471,
                ..V5_CAPABILITIES
            },
            Self::NaiDiffusion5Curated => ModelCapabilities {
                prompt_token_limit: 703,
                ..V5_CAPABILITIES
            },
            Self::NaiDiffusion45Full | Self::NaiDiffusion45Curated => V45_CAPABILITIES,
            Self::NaiDiffusion4Full | Self::NaiDiffusion4Curated => V4_CAPABILITIES,
            Self::NaiDiffusion3 => V3_CAPABILITIES,
            Self::NaiDiffusion3Furry => ModelCapabilities {
                default_scale: 6.2,
                ..V3_CAPABILITIES
            },
        }
    }

    pub const ALL: [Self; 8] = [
        Self::NaiDiffusion5Full,
        Self::NaiDiffusion5Curated,
        Self::NaiDiffusion45Full,
        Self::NaiDiffusion45Curated,
        Self::NaiDiffusion4Full,
        Self::NaiDiffusion4Curated,
        Self::NaiDiffusion3,
        Self::NaiDiffusion3Furry,
    ];
}

const V3_CAPABILITIES: ModelCapabilities = ModelCapabilities {
    prompt_structure: PromptStructure::Legacy,
    params_version: 3,
    default_steps: 23,
    default_scale: 5.0,
    max_characters: 0,
    supports_vibe_transfer: true,
    supports_encoded_vibe: false,
    supports_character_reference: false,
    supports_variety_boost: false,
    supports_inpainting: true,
    supports_smea: true,
    supports_dynamic_thresholding: true,
    uses_v5_extensions: false,
    supports_light_quality_preset: false,
    supports_transparent_background: false,
    variety_sigma_coefficient: None,
    prompt_token_limit: 225,
};

const V4_CAPABILITIES: ModelCapabilities = ModelCapabilities {
    prompt_structure: PromptStructure::V4,
    params_version: 3,
    default_steps: 23,
    default_scale: 5.5,
    max_characters: 6,
    supports_vibe_transfer: true,
    supports_encoded_vibe: true,
    supports_character_reference: false,
    supports_variety_boost: true,
    supports_inpainting: true,
    supports_smea: false,
    supports_dynamic_thresholding: false,
    uses_v5_extensions: false,
    supports_light_quality_preset: false,
    supports_transparent_background: false,
    variety_sigma_coefficient: Some(19.0),
    prompt_token_limit: 512,
};

const V45_CAPABILITIES: ModelCapabilities = ModelCapabilities {
    default_scale: 5.0,
    supports_character_reference: true,
    variety_sigma_coefficient: Some(58.0),
    ..V4_CAPABILITIES
};

const V5_CAPABILITIES: ModelCapabilities = ModelCapabilities {
    params_version: 4,
    default_steps: 23,
    default_scale: 7.0,
    supports_vibe_transfer: false,
    supports_encoded_vibe: false,
    supports_character_reference: false,
    supports_variety_boost: false,
    uses_v5_extensions: true,
    supports_light_quality_preset: true,
    supports_transparent_background: true,
    variety_sigma_coefficient: None,
    prompt_token_limit: 1471,
    ..V45_CAPABILITIES
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

impl ImageSize {
    #[must_use]
    pub const fn portrait() -> Self {
        Self {
            width: 832,
            height: 1216,
        }
    }

    #[must_use]
    pub const fn landscape() -> Self {
        Self {
            width: 1216,
            height: 832,
        }
    }

    #[must_use]
    pub const fn square() -> Self {
        Self {
            width: 1024,
            height: 1024,
        }
    }

    #[must_use]
    pub const fn large_portrait() -> Self {
        Self {
            width: 1024,
            height: 1536,
        }
    }

    #[must_use]
    pub const fn large_landscape() -> Self {
        Self {
            width: 1536,
            height: 1024,
        }
    }
}

impl Default for ImageSize {
    fn default() -> Self {
        Self::portrait()
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Sampler {
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

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum NoiseSchedule {
    Native,
    #[default]
    Karras,
    Exponential,
    Polyexponential,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum UcPreset {
    Heavy,
    #[default]
    Light,
    FurryFocus,
    HumanFocus,
    None,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Webp,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum StreamMode {
    #[default]
    Sse,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CharacterPosition {
    pub x: f32,
    pub y: f32,
}

impl Default for CharacterPosition {
    fn default() -> Self {
        Self { x: 0.5, y: 0.5 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Character {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub position: CharacterPosition,
    pub enabled: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CharacterReferenceType {
    Character,
    Style,
    CharacterAndStyle,
    Costume,
    Delta,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterReference {
    pub image: String,
    pub reference_type: CharacterReferenceType,
    pub fidelity: f32,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Img2ImgRequest {
    pub image: String,
    pub strength: f32,
    pub noise: f32,
    pub mask: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeReference {
    pub vibe_data_cache: String,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VibeTransferConfig {
    pub references: Vec<VibeReference>,
    pub strength: f32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum QualityPreset {
    #[default]
    Standard,
    Light,
    None,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateImageRequest {
    pub prompt: String,
    pub model: ImageModel,
    pub size: ImageSize,
    pub negative_prompt: Option<String>,
    pub quality: QualityPreset,
    pub transparent_background: bool,
    pub uc_preset: UcPreset,
    pub steps: u32,
    pub scale: f32,
    pub sampler: Sampler,
    pub noise_schedule: NoiseSchedule,
    pub seed: i64,
    pub n_samples: u32,
    pub cfg_rescale: f32,
    pub variety_boost: bool,
    pub img2img: Option<Img2ImgRequest>,
    pub vibe_transfer: Option<VibeTransferConfig>,
    pub character_references: Option<Vec<CharacterReference>>,
    pub characters: Option<Vec<Character>>,
    pub use_coords: Option<bool>,
    pub image_format: Option<ImageFormat>,
    pub strict_mode: bool,
}

impl Default for GenerateImageRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            model: ImageModel::default(),
            size: ImageSize::default(),
            negative_prompt: None,
            quality: QualityPreset::Standard,
            transparent_background: false,
            uc_preset: UcPreset::default(),
            steps: DEFAULT_STEPS,
            scale: DEFAULT_SCALE,
            sampler: Sampler::default(),
            noise_schedule: NoiseSchedule::default(),
            seed: 0,
            n_samples: 1,
            cfg_rescale: 0.0,
            variety_boost: false,
            img2img: None,
            vibe_transfer: None,
            character_references: None,
            characters: None,
            use_coords: None,
            image_format: None,
            strict_mode: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GenerateImageStreamRequest {
    pub base: GenerateImageRequest,
    pub stream: StreamMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedGeneratedImageMetadata {
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub seed: Option<i64>,
    pub metadata_json: String,
    pub warnings: Vec<GeneratedImageMetadataWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedImageMetadataWarning {
    InvalidCommentJson,
    InvalidTextChunk { keyword: String, message: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeneratedImageMetadata {
    Parsed(ParsedGeneratedImageMetadata),
    NotPresent,
    UnsupportedFormat,
    Invalid { message: String },
}

impl GeneratedImageMetadata {
    #[must_use]
    pub const fn parsed(&self) -> Option<&ParsedGeneratedImageMetadata> {
        match self {
            Self::Parsed(metadata) => Some(metadata),
            Self::NotPresent | Self::UnsupportedFormat | Self::Invalid { .. } => None,
        }
    }

    #[must_use]
    pub fn seed(&self) -> Option<i64> {
        self.parsed().and_then(|metadata| metadata.seed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
    pub metadata: GeneratedImageMetadata,
}

impl GeneratedImage {
    #[must_use]
    pub fn seed(&self) -> Option<i64> {
        self.metadata.seed()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerateImageResult {
    pub resolved_seed: i64,
    pub images: Vec<GeneratedImage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageStreamEvent {
    pub event_type: String,
    pub sample_index: u32,
    pub step_index: Option<u32>,
    pub generation_id: u32,
    pub sigma: Option<f32>,
    pub image: String,
}
