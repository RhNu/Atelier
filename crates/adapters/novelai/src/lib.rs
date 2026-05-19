use async_trait::async_trait;
use futures_util::StreamExt;
use nai_atelier_director::{
    DirectorResult, DirectorTool, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use nai_atelier_foundation::{NovelAiError, NovelAiErrorKind};
use nai_atelier_generation::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, ControlNetConfig,
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationResult,
    ImageFormat, ImageModel, ImageSize, ImageStreamEvent, ImageStreamResult, Img2ImgRequest,
    NoiseSchedule, NovelAiGenerationClient, Sampler, StreamMode, UcPreset,
};
use nai_atelier_secrets::{SecretsResult, SubscriptionClient, SubscriptionSummary};
use nai_atelier_vibe::{EncodeVibeRequest, EncodedVibe, NovelAiVibeClient, VibeModel, VibeResult};
use novelai_bridge as bridge;

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
    pub fn new(config: NovelAiBridgeConfig) -> Result<Self, NovelAiError> {
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
            .map_err(map_bridge_error)
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        let stream = self
            .client
            .generate_stream(to_bridge_stream_request(request))
            .await
            .map_err(map_bridge_error)?;
        Ok(Box::pin(stream.map(|item| {
            item.map(from_bridge_stream_chunk).map_err(map_bridge_error)
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
            .map_err(map_bridge_error)
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
            .map_err(map_bridge_error)
    }
}

#[async_trait]
impl<T> SubscriptionClient for NovelAiBridgeAdapter<T>
where
    T: bridge::Transport,
{
    async fn get_subscription(&self) -> SecretsResult<SubscriptionSummary> {
        self.client
            .get_subscription()
            .await
            .map(from_bridge_subscription)
            .map_err(map_bridge_error)
    }
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

fn map_bridge_error(error: bridge::BridgeError) -> NovelAiError {
    match error {
        bridge::BridgeError::InvalidRequest(error) => {
            NovelAiError::new(NovelAiErrorKind::InvalidRequest, error.to_string())
        }
        bridge::BridgeError::Api(error) => {
            let message = error.to_string();
            match error.kind {
                bridge::ApiErrorKind::InvalidRequest => {
                    NovelAiError::new(NovelAiErrorKind::InvalidRequest, message)
                        .with_status(error.status)
                }
                bridge::ApiErrorKind::AuthenticationFailed => {
                    NovelAiError::new(NovelAiErrorKind::Authentication, message)
                        .with_status(error.status)
                }
                bridge::ApiErrorKind::InsufficientCredit => {
                    NovelAiError::new(NovelAiErrorKind::InsufficientCredit, message)
                        .with_status(error.status)
                }
                bridge::ApiErrorKind::RequestConflict => {
                    NovelAiError::new(NovelAiErrorKind::RequestConflict, message)
                        .with_status(error.status)
                }
                bridge::ApiErrorKind::UnexpectedStatus => {
                    NovelAiError::new(NovelAiErrorKind::UnknownApi, message)
                        .with_status(error.status)
                }
                bridge::ApiErrorKind::RateLimited { retry_after } => {
                    let mut mapped = NovelAiError::new(NovelAiErrorKind::RateLimited, message)
                        .with_status(error.status);
                    if let Some(delay) = retry_after {
                        mapped = mapped.with_retry_after(delay);
                    }
                    mapped
                }
                bridge::ApiErrorKind::ServerError => {
                    NovelAiError::new(NovelAiErrorKind::ServiceUnavailable, message)
                        .with_status(error.status)
                }
            }
        }
        bridge::BridgeError::Transport(error) => {
            NovelAiError::new(NovelAiErrorKind::Transport, error.to_string())
        }
        bridge::BridgeError::Decode(error) => {
            NovelAiError::new(NovelAiErrorKind::Decode, error.to_string())
        }
        bridge::BridgeError::Metadata(error) => {
            NovelAiError::new(NovelAiErrorKind::Metadata, error.to_string())
        }
    }
}

fn to_bridge_model(model: ImageModel) -> bridge::Model {
    match model {
        ImageModel::NaiDiffusion45Full => bridge::Model::NaiDiffusion45Full,
        ImageModel::NaiDiffusion45Curated => bridge::Model::NaiDiffusion45Curated,
        ImageModel::NaiDiffusion4Full => bridge::Model::NaiDiffusion4Full,
        ImageModel::NaiDiffusion4Curated => bridge::Model::NaiDiffusion4Curated,
        ImageModel::NaiDiffusion3 => bridge::Model::NaiDiffusion3,
        ImageModel::NaiDiffusion3Furry => bridge::Model::NaiDiffusion3Furry,
    }
}

fn to_bridge_vibe_model(model: VibeModel) -> bridge::Model {
    match model {
        VibeModel::NaiDiffusion45Full => bridge::Model::NaiDiffusion45Full,
        VibeModel::NaiDiffusion45Curated => bridge::Model::NaiDiffusion45Curated,
        VibeModel::NaiDiffusion4Full => bridge::Model::NaiDiffusion4Full,
        VibeModel::NaiDiffusion4Curated => bridge::Model::NaiDiffusion4Curated,
        VibeModel::NaiDiffusion3 => bridge::Model::NaiDiffusion3,
        VibeModel::NaiDiffusion3Furry => bridge::Model::NaiDiffusion3Furry,
    }
}

fn to_bridge_size(size: ImageSize) -> bridge::ImageSize {
    bridge::ImageSize {
        width: size.width,
        height: size.height,
    }
}

fn to_bridge_sampler(sampler: Sampler) -> bridge::Sampler {
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

fn to_bridge_noise_schedule(schedule: NoiseSchedule) -> bridge::NoiseSchedule {
    match schedule {
        NoiseSchedule::Karras => bridge::NoiseSchedule::Karras,
        NoiseSchedule::Exponential => bridge::NoiseSchedule::Exponential,
        NoiseSchedule::Polyexponential => bridge::NoiseSchedule::Polyexponential,
    }
}

fn to_bridge_uc_preset(preset: UcPreset) -> bridge::UcPreset {
    match preset {
        UcPreset::Heavy => bridge::UcPreset::Heavy,
        UcPreset::Light => bridge::UcPreset::Light,
        UcPreset::FurryFocus => bridge::UcPreset::FurryFocus,
        UcPreset::HumanFocus => bridge::UcPreset::HumanFocus,
        UcPreset::None => bridge::UcPreset::None,
    }
}

fn to_bridge_image_format(format: ImageFormat) -> bridge::ImageFormat {
    match format {
        ImageFormat::Png => bridge::ImageFormat::Png,
        ImageFormat::Webp => bridge::ImageFormat::Webp,
    }
}

fn to_bridge_stream_mode(mode: StreamMode) -> bridge::StreamMode {
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

fn to_bridge_character_position(position: CharacterPosition) -> bridge::CharacterPosition {
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

fn to_bridge_character_reference_type(
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

fn to_bridge_director_tool(tool: DirectorTool) -> bridge::DirectorTool {
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
    use std::{io, time::Duration};

    use nai_atelier_foundation::NovelAiErrorKind;
    use nai_atelier_generation::{
        GenerateImageRequest, GeneratedImage, ImageModel, ImageSize, NoiseSchedule, Sampler,
        UcPreset,
    };
    use novelai_bridge::{ApiError, ApiErrorKind, BridgeError};

    use super::*;

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
        assert_eq!(invalid.kind, NovelAiErrorKind::InvalidRequest);

        let auth = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::AuthenticationFailed,
            status: 401,
            endpoint: "https://api.novelai.net/user/subscription".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert_eq!(auth.kind, NovelAiErrorKind::Authentication);

        let rate_limited = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::RateLimited {
                retry_after: Some(Duration::from_secs(3)),
            },
            status: 429,
            endpoint: "https://image.novelai.net/ai/generate-image".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert_eq!(rate_limited.kind, NovelAiErrorKind::RateLimited);
        assert_eq!(rate_limited.retry_after, Some(Duration::from_secs(3)));

        let insufficient_credit = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::InsufficientCredit,
            status: 402,
            endpoint: "https://image.novelai.net/ai/generate-image".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert_eq!(
            insufficient_credit.kind,
            NovelAiErrorKind::InsufficientCredit
        );

        let conflict = map_bridge_error(BridgeError::Api(ApiError {
            kind: ApiErrorKind::RequestConflict,
            status: 409,
            endpoint: "https://image.novelai.net/ai/generate-image".to_owned(),
            server_reason: None,
            raw_body: None,
        }));
        assert_eq!(conflict.kind, NovelAiErrorKind::RequestConflict);

        let transport = map_bridge_error(BridgeError::Transport(novelai_bridge::TransportError {
            operation: novelai_bridge::TransportOperation::SendRequest,
            endpoint: Some("https://image.novelai.net/ai/generate-image".to_owned()),
            source: Box::new(io::Error::other("network down")),
        }));
        assert_eq!(transport.kind, NovelAiErrorKind::Transport);

        let decode = map_bridge_error(BridgeError::Decode(novelai_bridge::DecodeError {
            target: novelai_bridge::DecodeTarget::JsonResponse,
            source: Box::new(io::Error::other("bad json")),
        }));
        assert_eq!(decode.kind, NovelAiErrorKind::Decode);

        let metadata = map_bridge_error(
            novelai_bridge::parse_png_metadata_from_bytes(b"not png").unwrap_err(),
        );
        assert_eq!(metadata.kind, NovelAiErrorKind::Metadata);
    }

    #[test]
    fn config_debug_redacts_api_key() {
        let config = NovelAiBridgeConfig::new("secret-token");

        let output = format!("{config:?}");

        assert!(output.contains("<redacted>"));
        assert!(!output.contains("secret-token"));
    }
}
