use async_trait::async_trait;
use atelier_director::{
    DirectorResult, DirectorTool, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use atelier_generation::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, GenerateImageRequest,
    GenerateImageResult, GenerateImageStreamRequest, GenerateImageStreamResult, GeneratedImage,
    GeneratedImageMetadata, GeneratedImageMetadataInspector, GeneratedImageMetadataWarning,
    GenerationResult, ImageFormat, ImageModel, ImageSize, ImageStreamEvent, Img2ImgRequest,
    NoiseSchedule, NovelAiGenerationClient, ParsedGeneratedImageMetadata, QualityPreset, Sampler,
    StreamMode, UcPreset, VibeTransferConfig,
};
use atelier_secrets::{
    SecretResolver, SecretValue, SecretsError, SubscriptionClient, SubscriptionProbeClient,
    SubscriptionResult, SubscriptionSummary, V5UsageStatus,
};
use atelier_vibe::{
    EmbeddedVibeDocumentExtractor, EncodeVibeRequest, EncodedVibe, NovelAiVibeClient,
    VibeDomainResult, VibeError, VibeModel, VibeResult,
};
use futures_util::StreamExt;
use novelai_bridge as bridge;

use crate::error::{
    NovelAiBridgeError, map_bridge_error, map_director_error, map_generation_error,
    map_subscription_error, map_vibe_error,
};

mod adapter;
mod bridge_client;
mod config;
mod factory;
mod mapping;
mod resolver;
mod subscription;

#[cfg(test)]
mod tests;

pub use adapter::{NovelAiBridgeAdapter, NovelAiEmbeddedVibeExtractor};
pub use config::NovelAiBridgeConfig;
pub use factory::{NovelAiClientFactory, ReqwestNovelAiClientFactory};
pub use resolver::ResolverBackedNovelAiAdapter;
pub use subscription::NovelAiSubscriptionProbeClient;

use mapping::{
    from_bridge_generated_image, from_bridge_generated_image_metadata, from_bridge_stream_chunk,
    from_bridge_subscription, map_secrets_error, to_bridge_director_request,
    to_bridge_encode_vibe_request, to_bridge_generate_request, to_bridge_stream_request,
};
