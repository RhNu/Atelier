//! `NovelAI` generation feature domain.

mod error;
mod estimate;
mod model;
mod normalize;
mod ports;
mod request_plan;

pub use error::{
    ClientApiErrorContext, ClientApiErrorReason, ClientDecodeContext, ClientDecodeTarget,
    ClientInvalidRequestContext, ClientInvalidRequestKind, ClientMetadataContext,
    ClientMetadataKind, ClientTransportContext, ClientTransportOperation, GenerationClientError,
    GenerationError, GenerationErrorKind,
};
pub use estimate::{AnlasEstimate, AnlasEstimateInput, estimate_anlas_cost};
pub use model::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, ControlNetConfig,
    ControlNetInput, GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, ImageFormat,
    ImageModel, ImageSize, ImageStreamEvent, Img2ImgRequest, NoiseSchedule, Sampler, StreamMode,
    UcPreset,
};
pub use normalize::normalize_generate_request;
pub use ports::{GenerationResult, ImageStreamResult, NovelAiGenerationClient};
pub use request_plan::{
    GenerationOutputMode, GenerationPlanContext, GenerationRequestPlan, SeedMode,
    plan_generation_request, plan_generation_stream_request,
};

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
