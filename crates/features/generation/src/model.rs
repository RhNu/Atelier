pub const DEFAULT_STEPS: u32 = 23;
pub const DEFAULT_SCALE: f32 = 5.0;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum ImageModel {
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
            Self::NaiDiffusion45Full => "nai-diffusion-4-5-full",
            Self::NaiDiffusion45Curated => "nai-diffusion-4-5-curated",
            Self::NaiDiffusion4Full => "nai-diffusion-4-full",
            Self::NaiDiffusion4Curated => "nai-diffusion-4-curated",
            Self::NaiDiffusion3 => "nai-diffusion-3",
            Self::NaiDiffusion3Furry => "nai-diffusion-3-furry",
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
    pub const fn is_v45(self) -> bool {
        matches!(self, Self::NaiDiffusion45Full | Self::NaiDiffusion45Curated)
    }

    #[must_use]
    pub const fn vibe_model_key(self) -> &'static str {
        match self {
            Self::NaiDiffusion45Full => "v4-5full",
            Self::NaiDiffusion45Curated => "v4-5curated",
            Self::NaiDiffusion4Full => "v4full",
            Self::NaiDiffusion4Curated => "v4curated",
            Self::NaiDiffusion3 => "v3",
            Self::NaiDiffusion3Furry => "v3furry",
        }
    }
}

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
    KDpmpp2sAncestral,
    KDpmppSde,
    Ddim,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum NoiseSchedule {
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
pub struct ControlNetInput {
    pub vibe_data_cache: String,
    pub info_extracted: f32,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlNetConfig {
    pub images: Vec<ControlNetInput>,
    pub strength: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerateImageRequest {
    pub prompt: String,
    pub model: ImageModel,
    pub size: ImageSize,
    pub negative_prompt: Option<String>,
    pub quality: bool,
    pub uc_preset: UcPreset,
    pub steps: u32,
    pub scale: f32,
    pub sampler: Sampler,
    pub noise_schedule: NoiseSchedule,
    pub seed: i64,
    pub n_samples: u32,
    pub cfg_rescale: f32,
    pub variety_boost: bool,
    pub i2i: Option<Img2ImgRequest>,
    pub controlnet: Option<ControlNetConfig>,
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
            quality: true,
            uc_preset: UcPreset::default(),
            steps: DEFAULT_STEPS,
            scale: DEFAULT_SCALE,
            sampler: Sampler::default(),
            noise_schedule: NoiseSchedule::default(),
            seed: 0,
            n_samples: 1,
            cfg_rescale: 0.0,
            variety_boost: false,
            i2i: None,
            controlnet: None,
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
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
    pub seed: Option<i64>,
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
