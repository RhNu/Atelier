//! `NovelAI` generation feature domain.

mod draft;
mod error;
mod estimate;
mod model;
mod normalize;
mod ports;
mod request_plan;

pub use draft::{
    GenerationDraftCharacter, GenerationDraftCharacterPositionMode, GenerationDraftError,
    GenerationDraftErrorKind, GenerationDraftI2i, GenerationDraftPreciseReference,
    GenerationDraftPromptState, GenerationDraftRepository, GenerationDraftResult,
    GenerationDraftSeedMode, GenerationDraftService, GenerationDraftSnapshot, GenerationDraftVibe,
    GenerationDraftVibeSlot,
};
pub use error::{
    ClientApiErrorContext, ClientApiErrorReason, ClientDecodeContext, ClientDecodeTarget,
    ClientInvalidRequestContext, ClientInvalidRequestKind, ClientMetadataContext,
    ClientMetadataKind, ClientTransportContext, ClientTransportOperation, GenerationClientError,
    GenerationError, GenerationErrorKind,
};
pub use estimate::{AnlasEstimate, AnlasEstimateStatus};
pub use model::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, GenerateImageRequest,
    GenerateImageResult, GenerateImageStreamRequest, GeneratedImage, GeneratedImageMetadata,
    GeneratedImageMetadataWarning, ImageFormat, ImageModel, ImageSize, ImageStreamEvent,
    Img2ImgRequest, ModelCapabilities, ModelDescriptor, NoiseSchedule,
    ParsedGeneratedImageMetadata, PromptStructure, PromptTokenCount, PromptTokenCountError,
    QualityPreset, Sampler, StreamMode, UcPreset, VibeReference, VibeTransferConfig,
    count_prompt_tokens,
};
pub use normalize::normalize_generate_request;
pub use ports::{
    CancellableImageStream, GenerateImageStreamResult, GeneratedImageMetadataInspector,
    GenerationResult, ImageStreamResult, NovelAiGenerationClient, passive_image_stream,
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
        let count = count_prompt_tokens(ImageModel::NaiDiffusion45Full, "1girl, blue hair")
            .expect("bundled T5 tokenizer should load");

        assert!(count.used > 0);
        assert_eq!(
            count.limit,
            ImageModel::NaiDiffusion45Full
                .capabilities()
                .prompt_token_limit
        );
    }
}
