use async_trait::async_trait;
use nai_atelier_foundation::NovelAiError;

pub type VibeResult<T> = Result<T, NovelAiError>;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum VibeModel {
    #[default]
    NaiDiffusion45Full,
    NaiDiffusion45Curated,
    NaiDiffusion4Full,
    NaiDiffusion4Curated,
    NaiDiffusion3,
    NaiDiffusion3Furry,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncodeVibeRequest {
    pub image: String,
    pub information_extracted: f32,
    pub model: VibeModel,
    pub strict_mode: bool,
}

impl Default for EncodeVibeRequest {
    fn default() -> Self {
        Self {
            image: String::new(),
            information_extracted: 1.0,
            model: VibeModel::default(),
            strict_mode: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedVibe {
    pub payload: String,
}

#[async_trait]
pub trait NovelAiVibeClient: Send + Sync {
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-vibe");
    }

    #[test]
    fn encode_vibe_request_defaults_to_strict_v45_full() {
        let request = EncodeVibeRequest::default();

        assert_eq!(request.model, VibeModel::NaiDiffusion45Full);
        assert!((request.information_extracted - 1.0).abs() < f32::EPSILON);
        assert!(request.strict_mode);
    }
}
