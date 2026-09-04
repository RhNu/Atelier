//! `NovelAI` image generation domain model.
//!
//! Model descriptor knowledge is owned by `novelai-bridge` and re-exported
//! here. Atelier keeps its own [`ImageModel`] because the model catalog is a
//! product concern (ordering, DTO and database encodings), but every
//! descriptor query is delegated to the bridge so there is exactly one model
//! registry in the workspace.

pub use novelai_bridge::{ModelDescriptor, PromptStructure};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CharacterPositionMode {
    Grid5x5,
    Freeform,
}

/// Atelier-facing projection of the bridge model descriptor.
///
/// Values are derived from the bridge registry on every call; this is not a
/// second writable capability table.
#[allow(clippy::struct_excessive_bools)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ModelCapabilities {
    pub prompt_structure: PromptStructure,
    pub params_version: u32,
    pub default_steps: u32,
    pub default_scale: f32,
    pub max_characters: u32,
    pub character_position_mode: Option<CharacterPositionMode>,
    pub can_position_one_character: bool,
    pub supports_vibe_transfer: bool,
    pub supports_encoded_vibe: bool,
    pub supports_character_reference: bool,
    pub supports_character_reference_inpainting: bool,
    pub supports_variety_boost: bool,
    pub supports_inpainting: bool,
    pub supports_streaming: bool,
    pub supports_smea: bool,
    pub supports_dynamic_thresholding: bool,
    pub uses_v5_extensions: bool,
    pub has_opus_usage_limit: bool,
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
    /// Returns the `novelai-bridge` model this catalog entry maps to.
    ///
    /// This is the single crossing point between the Atelier catalog and
    /// upstream `NovelAI` model knowledge.
    #[must_use]
    pub const fn bridge_model(self) -> novelai_bridge::Model {
        match self {
            Self::NaiDiffusion5Full => novelai_bridge::Model::NaiDiffusion5Full,
            Self::NaiDiffusion5Curated => novelai_bridge::Model::NaiDiffusion5Curated,
            Self::NaiDiffusion45Full => novelai_bridge::Model::NaiDiffusion45Full,
            Self::NaiDiffusion45Curated => novelai_bridge::Model::NaiDiffusion45Curated,
            Self::NaiDiffusion4Full => novelai_bridge::Model::NaiDiffusion4Full,
            Self::NaiDiffusion4Curated => novelai_bridge::Model::NaiDiffusion4Curated,
            Self::NaiDiffusion3 => novelai_bridge::Model::NaiDiffusion3,
            Self::NaiDiffusion3Furry => novelai_bridge::Model::NaiDiffusion3Furry,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.bridge_model().as_str()
    }

    #[must_use]
    pub const fn vibe_model_key(self) -> Option<&'static str> {
        self.bridge_model().vibe_model_key()
    }

    #[must_use]
    pub fn from_vibe_model_key(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|model| model.vibe_model_key() == Some(value))
    }

    #[must_use]
    pub const fn descriptor(self) -> &'static ModelDescriptor {
        self.bridge_model().descriptor()
    }

    #[must_use]
    pub const fn capabilities(self) -> ModelCapabilities {
        let descriptor = self.descriptor();
        let character_reference_inpainting = descriptor.resolve_references(
            novelai_bridge::GenerationMode::Inpainting,
            novelai_bridge::ReferenceIntent {
                vibe: false,
                character: true,
            },
        );
        ModelCapabilities {
            prompt_structure: descriptor.wire().prompt_structure(),
            params_version: descriptor.wire().params_version(),
            default_steps: descriptor.defaults().sampling.steps,
            default_scale: descriptor.defaults().sampling.scale,
            max_characters: descriptor.prompt().max_characters(),
            character_position_mode: match descriptor.prompt().characters {
                Some(profile) => match profile.positioning {
                    novelai_bridge::CharacterPositionProfile::Grid5x5 => {
                        Some(CharacterPositionMode::Grid5x5)
                    }
                    novelai_bridge::CharacterPositionProfile::Freeform => {
                        Some(CharacterPositionMode::Freeform)
                    }
                    _ => None,
                },
                None => None,
            },
            can_position_one_character: matches!(
                descriptor.prompt().characters,
                Some(novelai_bridge::CharacterPromptProfile {
                    positioning: novelai_bridge::CharacterPositionProfile::Freeform,
                    ..
                })
            ),
            supports_vibe_transfer: descriptor.support().vibe.is_some(),
            supports_encoded_vibe: descriptor.can_encode_vibe(),
            supports_character_reference: descriptor.support().character_reference.is_some(),
            supports_character_reference_inpainting: character_reference_inpainting
                .character
                .is_available(),
            supports_variety_boost: descriptor.sampling().variety.is_some(),
            supports_inpainting: descriptor.support().inpainting.is_some(),
            supports_streaming: descriptor.support().streaming,
            supports_smea: descriptor.sampling().smea,
            supports_dynamic_thresholding: descriptor.sampling().dynamic_thresholding,
            uses_v5_extensions: descriptor.wire().uses_v5_extensions(),
            has_opus_usage_limit: matches!(
                descriptor.pricing().opus_allowance,
                novelai_bridge::OpusAllowance::Metered
            ),
            supports_light_quality_preset: descriptor
                .prompt()
                .quality
                .supports(novelai_bridge::QualityPreset::Light),
            supports_transparent_background: descriptor.prompt().transparent_background,
            variety_sigma_coefficient: match descriptor.sampling().variety {
                Some(profile) => Some(profile.sigma_coefficient),
                None => None,
            },
            prompt_token_limit: descriptor.prompt().token_limit,
        }
    }

    #[must_use]
    pub const fn can_encode_vibe(self) -> bool {
        self.descriptor().can_encode_vibe()
    }

    #[must_use]
    pub const fn supports_light_quality_preset(self) -> bool {
        self.capabilities().supports_light_quality_preset
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
        let model = ImageModel::default();
        let defaults = model.descriptor().defaults();
        Self {
            prompt: String::new(),
            model,
            size: ImageSize::default(),
            negative_prompt: None,
            quality: QualityPreset::Standard,
            transparent_background: false,
            uc_preset: UcPreset::default(),
            steps: defaults.sampling.steps,
            scale: defaults.sampling.scale,
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
