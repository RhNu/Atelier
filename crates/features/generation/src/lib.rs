//! `NovelAI` generation feature domain.

mod draft;
mod error;
mod estimate;
mod model;
mod normalize;
mod ports;
mod prompt_tokens;
mod request_plan;

pub use draft::{
    GenerationDraftCharacter, GenerationDraftCharacterPositionMode, GenerationDraftError,
    GenerationDraftErrorKind, GenerationDraftFocusRegion, GenerationDraftI2i,
    GenerationDraftInpaintSession, GenerationDraftMaskDisplay, GenerationDraftMaskPattern,
    GenerationDraftPreciseReference, GenerationDraftPromptState, GenerationDraftReferenceInset,
    GenerationDraftRepository, GenerationDraftResult, GenerationDraftSeedMode,
    GenerationDraftService, GenerationDraftSnapshot, GenerationDraftVibe, GenerationDraftVibeSlot,
};
pub use error::{
    ClientApiErrorContext, ClientApiErrorReason, ClientDecodeContext, ClientDecodeTarget,
    ClientInvalidRequestContext, ClientInvalidRequestKind, ClientMetadataContext,
    ClientMetadataKind, ClientTransportContext, ClientTransportOperation, GenerationClientError,
    GenerationError, GenerationErrorKind,
};
pub use estimate::{AnlasEstimate, AnlasEstimateStatus};
pub use model::{
    Character, CharacterPosition, CharacterPositionMode, CharacterReference,
    CharacterReferenceType, GenerateImageRequest, GenerateImageResult, GenerateImageStreamRequest,
    GeneratedImage, GeneratedImageMetadata, GeneratedImageMetadataWarning, ImageFormat, ImageModel,
    ImageSize, ImageStreamEvent, Img2ImgRequest, InpaintRequest, ModelCapabilities,
    ModelDescriptor, NoiseSchedule, ParsedGeneratedImageMetadata, PromptStructure, QualityPreset,
    Sampler, StreamMode, UcPreset, VibeReference, VibeTransferConfig,
};
pub use normalize::normalize_generate_request;
pub use ports::{
    CancellableImageStream, GenerateImageStreamResult, GeneratedImageMetadataInspector,
    GenerationResult, ImageStreamResult, NovelAiGenerationClient, passive_image_stream,
};
pub use prompt_tokens::{
    CharacterPromptTokenUsage, PromptTokenCount, PromptTokenCountError, PromptTokenUsage,
    count_prompt_tokens,
};
pub use request_plan::{
    GenerationOutputMode, GenerationPlanContext, GenerationRequestPlan, SeedMode,
    plan_generation_request, plan_generation_stream_request,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "atelier-generation");
    }

    #[test]
    fn generate_request_defaults_are_novelai_oriented() {
        let request = GenerateImageRequest::default();

        assert_eq!(request.model, ImageModel::NaiDiffusion45Full);
        assert_eq!(request.size, ImageSize::portrait());
        assert_eq!(request.steps, 23);
        assert_eq!(request.n_samples, 1);
        assert_eq!(request.quality, QualityPreset::Standard);
    }

    #[test]
    fn prompt_token_count_uses_the_model_descriptor_limit() {
        let request = GenerateImageRequest {
            prompt: "1girl, blue hair".to_owned(),
            quality: QualityPreset::None,
            uc_preset: UcPreset::None,
            ..GenerateImageRequest::default()
        };
        let usage = count_prompt_tokens(&request).expect("bundled T5 tokenizer should load");

        assert!(usage.prompt.used > 0);
        assert_eq!(
            usage.prompt.limit,
            ImageModel::NaiDiffusion45Full
                .capabilities()
                .prompt_token_limit
        );
    }

    #[test]
    fn prompt_token_count_uses_effective_request_fields() {
        let request = GenerateImageRequest {
            prompt: "1girl".to_owned(),
            negative_prompt: Some("bad hands".to_owned()),
            quality: QualityPreset::Standard,
            uc_preset: UcPreset::Heavy,
            characters: Some(vec![
                Character {
                    prompt: "hero".to_owned(),
                    negative_prompt: Some("villain".to_owned()),
                    position: CharacterPosition::default(),
                    enabled: true,
                },
                Character {
                    prompt: "disabled".to_owned(),
                    negative_prompt: None,
                    position: CharacterPosition::default(),
                    enabled: false,
                },
            ]),
            ..GenerateImageRequest::default()
        };

        let usage = count_prompt_tokens(&request).expect("bundled T5 tokenizer should load");

        assert_eq!(usage.characters.len(), 1);
        assert_eq!(usage.characters[0].index, 0);
        assert!(usage.negative_prompt.used > 0);
    }

    #[test]
    fn furry_mode_prefix_is_included_in_prompt_token_count() {
        let request = GenerateImageRequest {
            prompt: "1girl".to_owned(),
            quality: QualityPreset::None,
            uc_preset: UcPreset::None,
            ..GenerateImageRequest::default()
        };
        let plain = count_prompt_tokens(&request).unwrap();
        let furry = count_prompt_tokens(&GenerateImageRequest {
            furry_mode: true,
            ..request
        })
        .unwrap();
        assert!(furry.prompt.used > plain.prompt.used);
    }
}
