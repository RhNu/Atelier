use async_trait::async_trait;
use futures_util::StreamExt;
use nai_atelier_director::{
    DirectorResult, DirectorTool, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use nai_atelier_generation::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, ControlNetConfig,
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageFormat, ImageModel, ImageSize, ImageStreamEvent, ImageStreamResult, Img2ImgRequest,
    NoiseSchedule, NovelAiGenerationClient, Sampler, StreamMode, UcPreset,
};
use nai_atelier_secrets::{
    SecretResolver, SecretValue, SecretsError, SubscriptionClient, SubscriptionProbeClient,
    SubscriptionResult, SubscriptionSummary,
};
use nai_atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeModel, VibeResult};
use novelai_bridge as bridge;

mod error;

pub use error::NovelAiBridgeError;

use error::{
    map_bridge_error, map_director_error, map_generation_error, map_subscription_error,
    map_vibe_error,
};

#[derive(Clone, PartialEq, Eq)]
pub struct NovelAiBridgeConfig {
    pub api_key: String,
    pub timeout_ms: u64,
}

impl std::fmt::Debug for NovelAiBridgeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NovelAiBridgeConfig")
            .field("api_key", &"<redacted>")
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

impl NovelAiBridgeConfig {
    #[must_use]
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            timeout_ms: bridge::DEFAULT_TIMEOUT_MS,
        }
    }
}

pub struct NovelAiBridgeAdapter<T: bridge::Transport = bridge::ReqwestTransport> {
    client: bridge::Client<T>,
}

impl NovelAiBridgeAdapter<bridge::ReqwestTransport> {
    /// Creates a bridge-backed adapter from an explicit API key.
    ///
    /// This deliberately does not read `NOVELAI_API_KEY`; secret resolution is
    /// owned by the future secrets/keyring adapters.
    /// # Errors
    /// Returns an error when the bridge client rejects the supplied configuration.
    pub fn new(config: NovelAiBridgeConfig) -> Result<Self, NovelAiBridgeError> {
        let options = bridge::ClientOptions {
            api_key_source: bridge::ApiKeySource::Inline {
                value: config.api_key.into(),
            },
            timeout_ms: config.timeout_ms,
            ..bridge::ClientOptions::default()
        };
        bridge::Client::new(options)
            .map(Self::from_client)
            .map_err(map_bridge_error)
    }
}

impl<T: bridge::Transport> NovelAiBridgeAdapter<T> {
    #[must_use]
    pub const fn from_client(client: bridge::Client<T>) -> Self {
        Self { client }
    }
}

pub trait NovelAiClientFactory: Clone + Send + Sync {
    type Client: NovelAiGenerationClient
        + NovelAiVibeClient
        + NovelAiDirectorClient
        + SubscriptionClient
        + Send
        + Sync;

    /// Creates a `NovelAI` client for one resolved secret value.
    ///
    /// # Errors
    /// Returns an error when the client cannot be constructed from the secret.
    fn create_client(&self, secret: SecretValue) -> Result<Self::Client, NovelAiBridgeError>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ReqwestNovelAiClientFactory {
    timeout_ms: u64,
}

impl Default for ReqwestNovelAiClientFactory {
    fn default() -> Self {
        Self {
            timeout_ms: bridge::DEFAULT_TIMEOUT_MS,
        }
    }
}

impl ReqwestNovelAiClientFactory {
    #[must_use]
    pub const fn new(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

impl NovelAiClientFactory for ReqwestNovelAiClientFactory {
    type Client = NovelAiBridgeAdapter<bridge::ReqwestTransport>;

    fn create_client(&self, secret: SecretValue) -> Result<Self::Client, NovelAiBridgeError> {
        NovelAiBridgeAdapter::new(NovelAiBridgeConfig {
            api_key: secret.expose_secret().to_owned(),
            timeout_ms: self.timeout_ms,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ResolverBackedNovelAiAdapter<R, F = ReqwestNovelAiClientFactory> {
    resolver: R,
    factory: F,
}

impl<R, F> ResolverBackedNovelAiAdapter<R, F> {
    #[must_use]
    pub const fn new(resolver: R, factory: F) -> Self {
        Self { resolver, factory }
    }
}

impl<R, F> ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn create_client(&self) -> Result<F::Client, NovelAiBridgeError> {
        let secret = self
            .resolver
            .resolve_active_secret()
            .await
            .map_err(|error| map_secrets_error(&error))?;
        self.factory.create_client(secret)
    }
}

#[async_trait]
impl<R, F> NovelAiGenerationClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        self.create_client()
            .await
            .map_err(map_generation_error)?
            .generate(request)
            .await
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        self.create_client()
            .await
            .map_err(map_generation_error)?
            .generate_stream(request)
            .await
    }
}

#[async_trait]
impl<R, F> NovelAiVibeClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        self.create_client()
            .await
            .map_err(map_vibe_error)?
            .encode_vibe(request)
            .await
    }
}

#[async_trait]
impl<R, F> NovelAiDirectorClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        self.create_client()
            .await
            .map_err(map_director_error)?
            .run_director_tool(request)
            .await
    }
}

#[async_trait]
impl<R, F> SubscriptionClient for ResolverBackedNovelAiAdapter<R, F>
where
    R: SecretResolver,
    F: NovelAiClientFactory,
{
    async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary> {
        self.create_client()
            .await
            .map_err(map_subscription_error)?
            .get_subscription()
            .await
    }
}

#[derive(Clone, Debug)]
pub struct NovelAiSubscriptionProbeClient<F = ReqwestNovelAiClientFactory> {
    factory: F,
}

impl<F> NovelAiSubscriptionProbeClient<F> {
    #[must_use]
    pub const fn new(factory: F) -> Self {
        Self { factory }
    }
}

#[async_trait]
impl<F> SubscriptionProbeClient for NovelAiSubscriptionProbeClient<F>
where
    F: NovelAiClientFactory,
{
    async fn probe_subscription(
        &self,
        secret: SecretValue,
    ) -> SubscriptionResult<SubscriptionSummary> {
        self.factory
            .create_client(secret)
            .map_err(map_subscription_error)?
            .get_subscription()
            .await
    }
}

#[async_trait]
impl<T> NovelAiGenerationClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        self.client
            .generate(to_bridge_generate_request(request))
            .await
            .map(|images| {
                images
                    .into_iter()
                    .map(from_bridge_generated_image)
                    .collect()
            })
            .map_err(|error| map_generation_error(map_bridge_error(error)))
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        let stream = self
            .client
            .generate_stream(to_bridge_stream_request(request))
            .await
            .map_err(|error| map_generation_error(map_bridge_error(error)))?;
        Ok(Box::pin(stream.map(|item| {
            item.map(from_bridge_stream_chunk)
                .map_err(|error| map_generation_error(map_bridge_error(error)))
        })))
    }
}

#[async_trait]
impl<T> NovelAiVibeClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        self.client
            .encode_vibe(to_bridge_encode_vibe_request(request))
            .await
            .map(|payload| EncodedVibe { payload })
            .map_err(|error| map_vibe_error(map_bridge_error(error)))
    }
}

#[async_trait]
impl<T> NovelAiDirectorClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn run_director_tool(
        &self,
        request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        self.client
            .run_director_tool(to_bridge_director_request(request))
            .await
            .map(|image| DirectorToolOutput {
                bytes: image.bytes,
                mime_type: image.mime_type,
                seed: image.seed,
            })
            .map_err(|error| map_director_error(map_bridge_error(error)))
    }
}

#[async_trait]
impl<T> SubscriptionClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary> {
        self.client
            .get_subscription()
            .await
            .map(from_bridge_subscription)
            .map_err(|error| map_subscription_error(map_bridge_error(error)))
    }
}

fn map_secrets_error(error: &SecretsError) -> NovelAiBridgeError {
    NovelAiBridgeError::credential(error.to_string())
}

fn to_bridge_generate_request(request: GenerateImageRequest) -> bridge::GenerateImageRequest {
    bridge::GenerateImageRequest {
        prompt: request.prompt,
        model: to_bridge_model(request.model),
        size: to_bridge_size(request.size),
        negative_prompt: request.negative_prompt,
        quality: request.quality,
        uc_preset: to_bridge_uc_preset(request.uc_preset),
        steps: request.steps,
        scale: request.scale,
        sampler: to_bridge_sampler(request.sampler),
        noise_schedule: to_bridge_noise_schedule(request.noise_schedule),
        seed: request.seed,
        n_samples: request.n_samples,
        cfg_rescale: request.cfg_rescale,
        variety_boost: request.variety_boost,
        i2i: request.i2i.map(to_bridge_i2i),
        controlnet: request.controlnet.map(to_bridge_controlnet),
        character_references: request.character_references.map(|items| {
            items
                .into_iter()
                .map(to_bridge_character_reference)
                .collect()
        }),
        characters: request
            .characters
            .map(|items| items.into_iter().map(to_bridge_character).collect()),
        use_coords: request.use_coords,
        image_format: request.image_format.map(to_bridge_image_format),
        strict_mode: request.strict_mode,
    }
}

fn to_bridge_stream_request(
    request: GenerateImageStreamRequest,
) -> bridge::GenerateImageStreamRequest {
    bridge::GenerateImageStreamRequest {
        base: to_bridge_generate_request(request.base),
        stream: to_bridge_stream_mode(request.stream),
    }
}

fn to_bridge_encode_vibe_request(request: EncodeVibeRequest) -> bridge::EncodeVibeRequest {
    bridge::EncodeVibeRequest {
        image: request.image,
        information_extracted: request.information_extracted,
        model: to_bridge_vibe_model(request.model),
        strict_mode: request.strict_mode,
    }
}

fn to_bridge_director_request(request: RunDirectorToolRequest) -> bridge::RunDirectorToolRequest {
    bridge::RunDirectorToolRequest {
        tool: to_bridge_director_tool(request.tool),
        image: request.image,
        prompt: request.prompt,
        defry: request.defry,
        strict_mode: request.strict_mode,
    }
}

fn from_bridge_generated_image(image: bridge::GeneratedImage) -> GeneratedImage {
    GeneratedImage {
        bytes: image.bytes,
        mime_type: image.mime_type,
        seed: image.seed,
    }
}

fn from_bridge_stream_chunk(chunk: bridge::ImageStreamChunk) -> ImageStreamEvent {
    ImageStreamEvent {
        event_type: chunk.event_type,
        sample_index: chunk.samp_ix,
        step_index: chunk.step_ix,
        generation_id: chunk.gen_id,
        sigma: chunk.sigma,
        image: chunk.image,
    }
}

fn from_bridge_subscription(subscription: bridge::SubscriptionInfo) -> SubscriptionSummary {
    SubscriptionSummary {
        anlas_balance: subscription.anlas_balance,
        is_opus: subscription.is_opus,
        tier: subscription.tier,
        tier_name: subscription.tier_name,
        expires_at_ms: subscription.expires_at_ms,
    }
}

const fn to_bridge_model(model: ImageModel) -> bridge::Model {
    match model {
        ImageModel::NaiDiffusion45Full => bridge::Model::NaiDiffusion45Full,
        ImageModel::NaiDiffusion45Curated => bridge::Model::NaiDiffusion45Curated,
        ImageModel::NaiDiffusion4Full => bridge::Model::NaiDiffusion4Full,
        ImageModel::NaiDiffusion4Curated => bridge::Model::NaiDiffusion4Curated,
        ImageModel::NaiDiffusion3 => bridge::Model::NaiDiffusion3,
        ImageModel::NaiDiffusion3Furry => bridge::Model::NaiDiffusion3Furry,
    }
}

const fn to_bridge_vibe_model(model: VibeModel) -> bridge::Model {
    match model {
        VibeModel::NaiDiffusion45Full => bridge::Model::NaiDiffusion45Full,
        VibeModel::NaiDiffusion45Curated => bridge::Model::NaiDiffusion45Curated,
        VibeModel::NaiDiffusion4Full => bridge::Model::NaiDiffusion4Full,
        VibeModel::NaiDiffusion4Curated => bridge::Model::NaiDiffusion4Curated,
        VibeModel::NaiDiffusion3 => bridge::Model::NaiDiffusion3,
        VibeModel::NaiDiffusion3Furry => bridge::Model::NaiDiffusion3Furry,
    }
}

const fn to_bridge_size(size: ImageSize) -> bridge::ImageSize {
    bridge::ImageSize {
        width: size.width,
        height: size.height,
    }
}

const fn to_bridge_sampler(sampler: Sampler) -> bridge::Sampler {
    match sampler {
        Sampler::KEuler => bridge::Sampler::KEuler,
        Sampler::KEulerAncestral => bridge::Sampler::KEulerAncestral,
        Sampler::KDpm2 => bridge::Sampler::KDpm2,
        Sampler::KDpm2Ancestral => bridge::Sampler::KDpm2Ancestral,
        Sampler::KDpmpp2m => bridge::Sampler::KDpmpp2m,
        Sampler::KDpmpp2sAncestral => bridge::Sampler::KDpmpp2sAncestral,
        Sampler::KDpmppSde => bridge::Sampler::KDpmppSde,
        Sampler::Ddim => bridge::Sampler::Ddim,
    }
}

const fn to_bridge_noise_schedule(schedule: NoiseSchedule) -> bridge::NoiseSchedule {
    match schedule {
        NoiseSchedule::Karras => bridge::NoiseSchedule::Karras,
        NoiseSchedule::Exponential => bridge::NoiseSchedule::Exponential,
        NoiseSchedule::Polyexponential => bridge::NoiseSchedule::Polyexponential,
    }
}

const fn to_bridge_uc_preset(preset: UcPreset) -> bridge::UcPreset {
    match preset {
        UcPreset::Heavy => bridge::UcPreset::Heavy,
        UcPreset::Light => bridge::UcPreset::Light,
        UcPreset::FurryFocus => bridge::UcPreset::FurryFocus,
        UcPreset::HumanFocus => bridge::UcPreset::HumanFocus,
        UcPreset::None => bridge::UcPreset::None,
    }
}

const fn to_bridge_image_format(format: ImageFormat) -> bridge::ImageFormat {
    match format {
        ImageFormat::Png => bridge::ImageFormat::Png,
        ImageFormat::Webp => bridge::ImageFormat::Webp,
    }
}

const fn to_bridge_stream_mode(mode: StreamMode) -> bridge::StreamMode {
    match mode {
        StreamMode::Sse => bridge::StreamMode::Sse,
    }
}

fn to_bridge_i2i(request: Img2ImgRequest) -> bridge::Img2ImgRequest {
    bridge::Img2ImgRequest {
        image: request.image,
        strength: request.strength,
        noise: request.noise,
        mask: request.mask,
    }
}

fn to_bridge_controlnet(config: ControlNetConfig) -> bridge::ControlNetConfig {
    bridge::ControlNetConfig {
        images: config
            .images
            .into_iter()
            .map(|input| bridge::ControlNetInput {
                vibe_data_cache: input.vibe_data_cache,
                info_extracted: input.info_extracted,
                strength: input.strength,
            })
            .collect(),
        strength: config.strength,
    }
}

const fn to_bridge_character_position(position: CharacterPosition) -> bridge::CharacterPosition {
    bridge::CharacterPosition {
        x: position.x,
        y: position.y,
    }
}

fn to_bridge_character(character: Character) -> bridge::Character {
    bridge::Character {
        prompt: character.prompt,
        negative_prompt: character.negative_prompt,
        position: to_bridge_character_position(character.position),
        enabled: character.enabled,
    }
}

const fn to_bridge_character_reference_type(
    reference_type: CharacterReferenceType,
) -> bridge::CharacterReferenceType {
    match reference_type {
        CharacterReferenceType::Character => bridge::CharacterReferenceType::Character,
        CharacterReferenceType::Style => bridge::CharacterReferenceType::Style,
        CharacterReferenceType::CharacterAndStyle => {
            bridge::CharacterReferenceType::CharacterAndStyle
        }
    }
}

fn to_bridge_character_reference(reference: CharacterReference) -> bridge::CharacterReference {
    bridge::CharacterReference {
        image: reference.image,
        reference_type: to_bridge_character_reference_type(reference.reference_type),
        fidelity: reference.fidelity,
        strength: reference.strength,
    }
}

const fn to_bridge_director_tool(tool: DirectorTool) -> bridge::DirectorTool {
    match tool {
        DirectorTool::Lineart => bridge::DirectorTool::Lineart,
        DirectorTool::Sketch => bridge::DirectorTool::Sketch,
        DirectorTool::BgRemoval => bridge::DirectorTool::BgRemoval,
        DirectorTool::Emotion => bridge::DirectorTool::Emotion,
        DirectorTool::Declutter => bridge::DirectorTool::Declutter,
        DirectorTool::Colorize => bridge::DirectorTool::Colorize,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::{io, time::Duration};

    use async_trait::async_trait;
    use futures_executor::block_on;
    use nai_atelier_generation::{
        ClientApiErrorReason as GenerationApiErrorReason,
        ClientInvalidRequestKind as GenerationInvalidRequestKind, GenerateImageRequest,
        GeneratedImage, GenerationClientError, ImageModel, ImageSize, NoiseSchedule,
        NovelAiGenerationClient, Sampler, UcPreset,
    };
    use nai_atelier_secrets::{
        SecretResolver, SecretValue, SecretsError, SecretsResult, SubscriptionClient,
        SubscriptionClientError, SubscriptionProbeClient, SubscriptionResult,
    };
    use novelai_bridge::{ApiError, ApiErrorKind, ApiErrorReason, BridgeError};

    use super::*;
    use crate::error::{
        BridgeApiErrorReason, BridgeDecodeTarget, BridgeInvalidRequestKind, BridgeMetadataKind,
        BridgeTransportOperation,
    };

    #[test]
    fn crate_metadata_is_available() {
        assert_eq!(env!("CARGO_PKG_NAME"), "nai-atelier-adapter-novelai");
    }

    #[test]
    fn maps_generation_request_to_bridge_request() {
        let request = GenerateImageRequest {
            prompt: "1girl".to_owned(),
            model: ImageModel::NaiDiffusion3,
            size: ImageSize {
                width: 1024,
                height: 1024,
            },
            negative_prompt: Some("lowres".to_owned()),
            quality: false,
            uc_preset: UcPreset::Heavy,
            steps: 28,
            scale: 6.5,
            sampler: Sampler::KDpmpp2m,
            noise_schedule: NoiseSchedule::Exponential,
            seed: 1234,
            n_samples: 2,
            cfg_rescale: 0.25,
            ..Default::default()
        };

        let bridge = to_bridge_generate_request(request);

        assert_eq!(bridge.prompt, "1girl");
        assert_eq!(bridge.model, novelai_bridge::Model::NaiDiffusion3);
        assert_eq!(bridge.size.width, 1024);
        assert_eq!(bridge.uc_preset, novelai_bridge::UcPreset::Heavy);
        assert_eq!(bridge.sampler, novelai_bridge::Sampler::KDpmpp2m);
        assert_eq!(
            bridge.noise_schedule,
            novelai_bridge::NoiseSchedule::Exponential
        );
        assert_eq!(bridge.n_samples, 2);
    }

    #[test]
    fn maps_bridge_generated_image_to_domain() {
        let image = from_bridge_generated_image(novelai_bridge::GeneratedImage {
            bytes: vec![1, 2, 3],
            mime_type: Some("image/png".to_owned()),
            seed: Some(99),
        });

        assert_eq!(
            image,
            GeneratedImage {
                bytes: vec![1, 2, 3],
                mime_type: Some("image/png".to_owned()),
                seed: Some(99),
            }
        );
    }

    #[test]
    fn maps_bridge_error_categories() {
        let invalid = map_bridge_error(BridgeError::InvalidRequest(
            novelai_bridge::InvalidRequest::MissingConfiguration {
                name: "api_key".to_owned(),
            },
        ));
        match invalid {
            NovelAiBridgeError::InvalidRequest {
                status: None,
                context: Some(context),
                ..
            } => {
                assert_eq!(context.kind, BridgeInvalidRequestKind::MissingConfiguration);
                assert_eq!(context.name.as_deref(), Some("api_key"));
            }
            other => panic!("unexpected invalid request mapping: {other:?}"),
        }

        let auth = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::AuthenticationFailed,
            status: 401,
            endpoint: "https://api.novelai.net/user/subscription".to_owned(),
            server_reason: Some(ApiErrorReason::Detail("bad key".to_owned())),
            raw_body: Some("{\"detail\":\"bad key\"}".to_owned()),
        }));
        match auth {
            NovelAiBridgeError::Authentication {
                status: Some(401),
                context: Some(context),
                ..
            } => {
                assert_eq!(
                    context.endpoint,
                    "https://api.novelai.net/user/subscription"
                );
                assert_eq!(
                    context.server_reason,
                    Some(BridgeApiErrorReason::Detail("bad key".to_owned()))
                );
                assert_eq!(
                    context.raw_body.as_deref(),
                    Some("{\"detail\":\"bad key\"}")
                );
            }
            other => panic!("unexpected auth mapping: {other:?}"),
        }

        let rate_limited = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::RateLimited {
                retry_after: Some(Duration::from_secs(3)),
            },
            status: 429,
            endpoint: "https://image.novelai.net/ai/generate-image".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert!(matches!(
            rate_limited,
            NovelAiBridgeError::RateLimited {
                status: 429,
                retry_after: Some(delay),
                ..
            } if delay == Duration::from_secs(3)
        ));

        let insufficient_credit = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::InsufficientCredit,
            status: 402,
            endpoint: "https://image.novelai.net/ai/generate-image".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert!(matches!(
            insufficient_credit,
            NovelAiBridgeError::InsufficientCredit {
                status: Some(402),
                ..
            }
        ));

        let conflict = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::RequestConflict,
            status: 409,
            endpoint: "https://image.novelai.net/ai/generate-image".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert!(matches!(
            conflict,
            NovelAiBridgeError::RequestConflict {
                status: Some(409),
                ..
            }
        ));
    }

    #[test]
    fn maps_bridge_transport_decode_and_metadata_contexts() {
        let transport = map_bridge_error(BridgeError::Transport(novelai_bridge::TransportError {
            operation: novelai_bridge::TransportOperation::SendRequest,
            endpoint: Some("https://image.novelai.net/ai/generate-image".to_owned()),
            source: Box::new(io::Error::other("network down")),
        }));
        match transport {
            NovelAiBridgeError::Transport {
                context: Some(context),
                ..
            } => {
                assert_eq!(context.operation, BridgeTransportOperation::SendRequest);
                assert_eq!(
                    context.endpoint.as_deref(),
                    Some("https://image.novelai.net/ai/generate-image")
                );
                assert_eq!(context.source, "network down");
            }
            other => panic!("unexpected transport mapping: {other:?}"),
        }

        let decode = map_bridge_error(BridgeError::Decode(novelai_bridge::DecodeError {
            target: novelai_bridge::DecodeTarget::JsonResponse,
            source: Box::new(io::Error::other("bad json")),
        }));
        match decode {
            NovelAiBridgeError::Decode {
                context: Some(context),
                ..
            } => {
                assert_eq!(context.target, BridgeDecodeTarget::JsonResponse);
                assert_eq!(context.source, "bad json");
            }
            other => panic!("unexpected decode mapping: {other:?}"),
        }

        let metadata = map_bridge_error(
            novelai_bridge::parse_png_metadata_from_bytes(b"not png").unwrap_err(),
        );
        match metadata {
            NovelAiBridgeError::Metadata {
                context: Some(context),
                ..
            } => {
                assert_eq!(context.kind, BridgeMetadataKind::InvalidPngPayload);
                assert_eq!(context.field, "metadata.image");
            }
            other => panic!("unexpected metadata mapping: {other:?}"),
        }
    }

    #[test]
    fn maps_bridge_rate_limit_to_feature_client_errors_without_losing_delay() {
        let bridge_error =
            NovelAiBridgeError::rate_limited(429, Some(Duration::from_secs(11)), "slow down");

        assert!(matches!(
            map_generation_error(bridge_error.clone()),
            GenerationClientError::RateLimited {
                status: 429,
                retry_after: Some(delay),
                ..
            } if delay == Duration::from_secs(11)
        ));
        assert!(matches!(
            map_subscription_error(bridge_error),
            SubscriptionClientError::RateLimited {
                status: 429,
                retry_after: Some(delay),
                ..
            } if delay == Duration::from_secs(11)
        ));
    }

    #[test]
    fn maps_bridge_context_to_feature_client_errors_without_flattening() {
        let invalid = map_generation_error(map_bridge_error(BridgeError::InvalidRequest(
            novelai_bridge::InvalidRequest::NumericOutOfRange {
                field: "scale".to_owned(),
                value: 99.0,
                min: 0.0,
                max: 10.0,
            },
        )));
        match invalid {
            GenerationClientError::InvalidRequest {
                context: Some(context),
                ..
            } => {
                assert_eq!(
                    context.kind,
                    GenerationInvalidRequestKind::NumericOutOfRange
                );
                assert_eq!(context.field.as_deref(), Some("scale"));
                assert_eq!(context.value.as_deref(), Some("99"));
                assert_eq!(context.max.as_deref(), Some("10"));
            }
            other => panic!("unexpected invalid feature error mapping: {other:?}"),
        }

        let auth = map_generation_error(map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::AuthenticationFailed,
            status: 401,
            endpoint: "https://api.novelai.net/user/subscription".to_owned(),
            server_reason: Some(ApiErrorReason::ErrorMessage("expired".to_owned())),
            raw_body: Some("{\"error\":{\"message\":\"expired\"}}".to_owned()),
        })));
        match auth {
            GenerationClientError::Authentication {
                context: Some(context),
                ..
            } => {
                assert_eq!(
                    context.endpoint,
                    "https://api.novelai.net/user/subscription"
                );
                assert_eq!(
                    context.server_reason,
                    Some(GenerationApiErrorReason::ErrorMessage("expired".to_owned()))
                );
                assert_eq!(
                    context.raw_body.as_deref(),
                    Some("{\"error\":{\"message\":\"expired\"}}")
                );
            }
            other => panic!("unexpected auth feature error mapping: {other:?}"),
        }
    }

    #[test]
    fn config_debug_redacts_api_key() {
        let config = NovelAiBridgeConfig::new("secret-token");

        let output = format!("{config:?}");

        assert!(output.contains("<redacted>"));
        assert!(!output.contains("secret-token"));
    }

    #[test]
    fn resolver_backed_generation_resolves_active_secret_before_calling_client() {
        block_on(async {
            let resolver = FakeResolver::active("active-secret");
            let factory = RecordingFactory::default();
            let adapter = ResolverBackedNovelAiAdapter::new(resolver, factory.clone());

            adapter
                .generate(GenerateImageRequest {
                    prompt: "1girl".to_owned(),
                    ..Default::default()
                })
                .await
                .unwrap();

            assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        });
    }

    #[test]
    fn resolver_backed_generation_maps_missing_active_key_without_calling_client() {
        block_on(async {
            let resolver = FakeResolver::missing_active_key();
            let factory = RecordingFactory::default();
            let adapter = ResolverBackedNovelAiAdapter::new(resolver, factory.clone());

            let error = adapter
                .generate(GenerateImageRequest {
                    prompt: "1girl".to_owned(),
                    ..Default::default()
                })
                .await
                .unwrap_err();

            assert!(matches!(error, GenerationClientError::Credential { .. }));
            assert!(factory.secrets().is_empty());
        });
    }

    #[test]
    fn explicit_subscription_probe_uses_supplied_secret() {
        block_on(async {
            let factory = RecordingFactory::default();
            let probe = NovelAiSubscriptionProbeClient::new(factory.clone());

            let summary = probe
                .probe_subscription(SecretValue::new("probe-secret"))
                .await
                .unwrap();

            assert_eq!(summary.anlas_balance, 7);
            assert_eq!(factory.secrets(), vec!["probe-secret".to_owned()]);
        });
    }

    #[derive(Clone)]
    struct FakeResolver {
        result: SecretsResult<SecretValue>,
    }

    impl FakeResolver {
        fn active(value: &str) -> Self {
            Self {
                result: Ok(SecretValue::new(value)),
            }
        }

        fn missing_active_key() -> Self {
            Self {
                result: Err(SecretsError::missing_active_key()),
            }
        }
    }

    #[async_trait]
    impl SecretResolver for FakeResolver {
        async fn resolve_active_secret(&self) -> SecretsResult<SecretValue> {
            self.result.clone()
        }
    }

    #[derive(Clone, Default)]
    struct RecordingFactory {
        secrets: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingFactory {
        fn secrets(&self) -> Vec<String> {
            self.secrets.lock().unwrap().clone()
        }
    }

    impl NovelAiClientFactory for RecordingFactory {
        type Client = RecordingClient;

        fn create_client(&self, secret: SecretValue) -> Result<Self::Client, NovelAiBridgeError> {
            self.secrets
                .lock()
                .unwrap()
                .push(secret.expose_secret().to_owned());
            Ok(RecordingClient)
        }
    }

    #[derive(Clone)]
    struct RecordingClient;

    #[async_trait]
    impl NovelAiGenerationClient for RecordingClient {
        async fn generate(
            &self,
            _request: GenerateImageRequest,
        ) -> GenerationResult<Vec<GeneratedImage>> {
            Ok(vec![GeneratedImage {
                bytes: vec![1, 2, 3],
                mime_type: Some("image/png".to_owned()),
                seed: Some(1),
            }])
        }

        async fn generate_stream(
            &self,
            _request: GenerateImageStreamRequest,
        ) -> GenerationResult<ImageStreamResult> {
            Ok(Box::pin(futures_util::stream::empty()))
        }
    }

    #[async_trait]
    impl NovelAiVibeClient for RecordingClient {
        async fn encode_vibe(&self, _request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
            Ok(EncodedVibe {
                payload: "encoded".to_owned(),
            })
        }
    }

    #[async_trait]
    impl NovelAiDirectorClient for RecordingClient {
        async fn run_director_tool(
            &self,
            _request: RunDirectorToolRequest,
        ) -> DirectorResult<DirectorToolOutput> {
            Ok(DirectorToolOutput {
                bytes: vec![4, 5, 6],
                mime_type: Some("image/png".to_owned()),
                seed: Some(2),
            })
        }
    }

    #[async_trait]
    impl SubscriptionClient for RecordingClient {
        async fn get_subscription(&self) -> SubscriptionResult<SubscriptionSummary> {
            Ok(SubscriptionSummary {
                anlas_balance: 7,
                is_opus: false,
                tier: 1,
                tier_name: "tablet".to_owned(),
                expires_at_ms: None,
            })
        }
    }
}
