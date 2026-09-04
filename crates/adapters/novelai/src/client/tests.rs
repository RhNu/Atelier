use super::*;

use std::sync::{Arc, Mutex};
use std::{
    io::{self, Cursor},
    time::Duration,
};

use async_trait::async_trait;
use atelier_generation::{
    ClientApiErrorReason as GenerationApiErrorReason,
    ClientInvalidRequestKind as GenerationInvalidRequestKind, GenerateImageRequest, GeneratedImage,
    GenerationClientError, ImageModel, ImageSize, InpaintRequest, NoiseSchedule,
    NovelAiGenerationClient, QualityPreset, Sampler, UcPreset,
};
use atelier_secrets::{
    SecretResolver, SecretValue, SecretsError, SecretsResult, SubscriptionClient,
    SubscriptionClientError, SubscriptionProbeClient, SubscriptionResult,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_executor::block_on;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use novelai_bridge::{ApiError, ApiErrorKind, ApiErrorReason, BridgeError};
use serde_json::{Value, json};

use crate::error::{
    BridgeApiErrorReason, BridgeDecodeTarget, BridgeInvalidRequestKind, BridgeMetadataKind,
    BridgeTransportOperation,
};

#[test]
fn crate_metadata_is_available() {
    assert_eq!(env!("CARGO_PKG_NAME"), "atelier-adapter-novelai");
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
        quality: QualityPreset::None,
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

    let bridge = to_bridge_generate_request(request).unwrap();

    assert_eq!(bridge.core.prompt, "1girl");
    assert_eq!(bridge.core.model, novelai_bridge::Model::NaiDiffusion3);
    assert_eq!(bridge.core.size.width, 1024);
    assert_eq!(bridge.core.uc_preset, novelai_bridge::UcPreset::Heavy);
    assert_eq!(bridge.sampling.sampler, novelai_bridge::Sampler::KDpmpp2m);
    assert_eq!(
        bridge.sampling.noise_schedule,
        novelai_bridge::NoiseSchedule::Exponential
    );
    assert_eq!(bridge.core.n_samples, 2);
}

#[test]
fn maps_v5_extensions_and_rc1_enum_additions() {
    let request = GenerateImageRequest {
        prompt: "a natural-language scene".to_owned(),
        model: ImageModel::NaiDiffusion5Curated,
        quality: QualityPreset::Light,
        transparent_background: true,
        sampler: Sampler::DdimV3,
        noise_schedule: NoiseSchedule::Native,
        img2img: Some(Img2ImgRequest {
            image: png_base64(2, 1),
            strength: 0.6,
            noise: 0.1,
            inpaint: Some(InpaintRequest {
                region_to_replace: png_base64(2, 1),
            }),
        }),
        character_references: Some(vec![atelier_generation::CharacterReference {
            image: png_base64(2, 1),
            reference_type: atelier_generation::CharacterReferenceType::Costume,
            fidelity: 0.5,
            strength: 0.75,
        }]),
        ..Default::default()
    };

    let bridge = to_bridge_generate_request(request).unwrap();

    assert_eq!(
        bridge.core.model,
        novelai_bridge::Model::NaiDiffusion5Curated
    );
    assert_eq!(bridge.core.quality, novelai_bridge::QualityPreset::Light);
    assert!(bridge.core.transparent_background);
    assert_eq!(bridge.sampling.sampler, novelai_bridge::Sampler::DdimV3);
    assert!(
        bridge
            .img2img
            .as_ref()
            .and_then(|value| value.mask.as_ref())
            .is_some()
    );
    assert_eq!(
        bridge.sampling.noise_schedule,
        novelai_bridge::NoiseSchedule::Native
    );
    assert_eq!(
        bridge.references.unwrap().character_references[0].reference_type,
        novelai_bridge::CharacterReferenceType::Costume
    );
    assert_eq!(
        super::mapping::to_bridge_sampler(Sampler::KDpmpp2mSde),
        novelai_bridge::Sampler::KDpmpp2mSde
    );
    assert_eq!(
        super::mapping::to_bridge_character_reference_type(
            atelier_generation::CharacterReferenceType::Delta
        ),
        novelai_bridge::CharacterReferenceType::Delta
    );
}

#[test]
fn maps_bridge_generated_image_to_domain() {
    let image = from_bridge_generated_image(novelai_bridge::GeneratedImage {
        bytes: vec![1, 2, 3],
        mime_type: Some("image/png".to_owned()),
        metadata: novelai_bridge::GeneratedImageMetadata::Parsed(Box::new(
            novelai_bridge::PngMetadata {
                prompt: Some("1girl".to_owned()),
                seed: Some(99),
                ..novelai_bridge::PngMetadata::default()
            },
        )),
    });

    assert_eq!(image.bytes, vec![1, 2, 3]);
    assert_eq!(image.mime_type.as_deref(), Some("image/png"));
    assert_eq!(image.seed(), Some(99));
    assert_eq!(
        image
            .metadata
            .parsed()
            .and_then(|metadata| metadata.prompt.as_deref()),
        Some("1girl")
    );
}

#[test]
fn maps_v5_subscription_allowance_without_anlas_inference() {
    let summary = from_bridge_subscription(novelai_bridge::SubscriptionInfo {
        anlas_balance: 17,
        is_opus: true,
        subscription_active: true,
        tier: 3,
        tier_name: "opus".to_owned(),
        expires_at_ms: None,
        v5_usage: Some(novelai_bridge::V5UsageStatus {
            is_negative: true,
            percent: 12,
            seconds_until_next_percent: 30,
        }),
    });
    assert_eq!(summary.anlas_balance, 17);
    assert!(summary.subscription_active);
    assert_eq!(summary.v5_usage.unwrap().percent, 12);
    assert!(summary.v5_usage.unwrap().is_negative);
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

    let auth = map_bridge_error(BridgeError::Api(api_error(
        ApiErrorKind::AuthenticationFailed,
        401,
        "https://api.novelai.net/user/subscription",
        Some(ApiErrorReason::Detail("bad key".to_owned())),
        Some("{\"detail\":\"bad key\"}"),
    )));
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

    let rate_limited = map_bridge_error(BridgeError::Api(api_error(
        ApiErrorKind::RateLimited {
            retry_after: Some(Duration::from_secs(3)),
        },
        429,
        "https://image.novelai.net/ai/generate-image",
        None,
        None,
    )));
    assert!(matches!(
        rate_limited,
        NovelAiBridgeError::RateLimited {
            status: 429,
            retry_after: Some(delay),
            ..
        } if delay == Duration::from_secs(3)
    ));

    let insufficient_credit = map_bridge_error(BridgeError::Api(api_error(
        ApiErrorKind::InsufficientCredit,
        402,
        "https://image.novelai.net/ai/generate-image",
        None,
        None,
    )));
    assert!(matches!(
        insufficient_credit,
        NovelAiBridgeError::InsufficientCredit {
            status: Some(402),
            ..
        }
    ));

    let conflict = map_bridge_error(BridgeError::Api(api_error(
        ApiErrorKind::RequestConflict,
        409,
        "https://image.novelai.net/ai/generate-image",
        None,
        None,
    )));
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
    let transport = map_bridge_error(BridgeError::Transport(novelai_bridge::TransportError::new(
        novelai_bridge::TransportOperation::SendRequest,
        Some("https://image.novelai.net/ai/generate-image".to_owned()),
        Box::new(io::Error::other("network down")),
    )));
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

    let decode = map_bridge_error(BridgeError::Decode(novelai_bridge::DecodeError::new(
        novelai_bridge::DecodeTarget::JsonResponse,
        Box::new(io::Error::other("bad json")),
    )));
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

    let metadata =
        map_bridge_error(novelai_bridge::parse_png_metadata_from_bytes(b"not png").unwrap_err());
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

    let auth = map_generation_error(map_bridge_error(BridgeError::Api(api_error(
        ApiErrorKind::AuthenticationFailed,
        401,
        "https://api.novelai.net/user/subscription",
        Some(ApiErrorReason::ErrorMessage("expired".to_owned())),
        Some("{\"error\":{\"message\":\"expired\"}}"),
    ))));
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
fn director_adapter_normalizes_lineart_options_before_bridge_validation() {
    block_on(async {
        let response = png_bytes(1, 1);
        let transport = RecordingDirectorTransport::new(response.clone());
        let adapter =
            NovelAiBridgeAdapter::from_client(bridge::Client::with_transport(transport.clone()));

        let output = adapter
            .run_director_tool(RunDirectorToolRequest {
                tool: DirectorTool::Lineart,
                image: png_base64(2, 1),
                prompt: Some(" clean lines ".to_owned()),
                defry: Some(2),
                strict_mode: true,
            })
            .await
            .unwrap();

        assert_eq!(output.bytes, response);
        let body = transport.last_body().expect("director request body");
        assert_eq!(body["req_type"], json!("lineart"));
        assert!(body.get("prompt").is_none());
        assert!(body.get("defry").is_none());
    });
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
    ) -> GenerationResult<GenerateImageResult> {
        Ok(GenerateImageResult {
            resolved_seed: 1,
            images: vec![GeneratedImage {
                bytes: vec![1, 2, 3],
                mime_type: Some("image/png".to_owned()),
                metadata: GeneratedImageMetadata::NotPresent,
            }],
        })
    }

    async fn generate_stream(
        &self,
        _request: GenerateImageStreamRequest,
    ) -> GenerationResult<GenerateImageStreamResult> {
        Ok(GenerateImageStreamResult {
            resolved_seed: 1,
            stream: atelier_generation::passive_image_stream(Box::pin(
                futures_util::stream::empty(),
            )),
        })
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
            subscription_active: false,
            tier: 1,
            tier_name: "tablet".to_owned(),
            expires_at_ms: None,
            v5_usage: None,
        })
    }
}

#[derive(Clone)]
struct RecordingDirectorTransport {
    bytes_response: Arc<Vec<u8>>,
    last_body: Arc<Mutex<Option<Value>>>,
}

impl RecordingDirectorTransport {
    fn new(bytes_response: Vec<u8>) -> Self {
        Self {
            bytes_response: Arc::new(bytes_response),
            last_body: Arc::default(),
        }
    }

    fn last_body(&self) -> Option<Value> {
        self.last_body.lock().unwrap().clone()
    }
}

#[async_trait]
impl bridge::Transport for RecordingDirectorTransport {
    async fn execute(
        &self,
        request: bridge::TransportRequest,
    ) -> Result<bridge::TransportResponse, bridge::TransportError> {
        let body = request.body.expect("director request body");
        *self.last_body.lock().unwrap() = Some(serde_json::from_slice(&body).unwrap());
        Ok(bridge::TransportResponse {
            status: 200,
            headers: Vec::new(),
            body: (*self.bytes_response).clone(),
        })
    }
}

fn api_error(
    kind: ApiErrorKind,
    status: u16,
    endpoint: &str,
    server_reason: Option<ApiErrorReason>,
    raw_body: Option<&str>,
) -> ApiError {
    let mut error = ApiError::new(kind, status, endpoint);
    error.server_reason = server_reason;
    error.raw_body = raw_body.map(str::to_owned);
    error
}

fn png_base64(width: u32, height: u32) -> String {
    STANDARD.encode(png_bytes(width, height))
}

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_fn(width, height, |x, y| {
        Rgba([
            u8::try_from(x % 256).unwrap(),
            u8::try_from(y % 256).unwrap(),
            255,
            255,
        ])
    }));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
}
