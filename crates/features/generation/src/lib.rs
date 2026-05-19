use std::pin::Pin;

use async_trait::async_trait;
use futures_core::Stream;
use nai_atelier_foundation::NovelAiError;

pub type GenerationResult<T> = Result<T, NovelAiError>;
pub type ImageStreamResult =
    Pin<Box<dyn Stream<Item = GenerationResult<ImageStreamEvent>> + Send + 'static>>;

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
            steps: 23,
            scale: 5.0,
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

#[async_trait]
pub trait NovelAiGenerationClient: Send + Sync {
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>>;

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-generation");
    }

    #[test]
    fn generate_request_defaults_are_novelai_oriented() {
        let request = GenerateImageRequest::default();

        assert_eq!(request.model, ImageModel::NaiDiffusion45Full);
        assert_eq!(request.size, ImageSize::portrait());
        assert_eq!(request.steps, 23);
        assert_eq!(request.n_samples, 1);
        assert!(request.quality);
    }
}
