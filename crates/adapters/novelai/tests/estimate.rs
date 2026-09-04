use atelier_adapter_novelai::estimate_anlas_cost as try_estimate_anlas_cost;
use atelier_generation::{
    AnlasEstimate, AnlasEstimateStatus, CharacterReference, CharacterReferenceType,
    GenerateImageRequest, GenerationPlanContext, ImageModel, ImageSize, Img2ImgRequest,
    VibeReference, VibeTransferConfig,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
use std::io::Cursor;

fn estimate_anlas_cost(
    request: &GenerateImageRequest,
    context: GenerationPlanContext,
) -> AnlasEstimate {
    try_estimate_anlas_cost(request, context).unwrap()
}

fn png_base64() -> String {
    let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 255])));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png).unwrap();
    STANDARD.encode(bytes.into_inner())
}

/// 832x1216 at 23 steps, the Atelier default canvas.
fn base_request(model: ImageModel) -> GenerateImageRequest {
    GenerateImageRequest {
        prompt: "1girl".to_owned(),
        model,
        size: ImageSize {
            width: 832,
            height: 1216,
        },
        steps: 23,
        n_samples: 1,
        ..GenerateImageRequest::default()
    }
}

fn opus_context() -> GenerationPlanContext {
    GenerationPlanContext {
        tier: 3,
        subscription_active: true,
        ..GenerationPlanContext::default()
    }
}

#[test]
fn v5_applies_the_model_cost_multiplier() {
    let v45 = estimate_anlas_cost(
        &base_request(ImageModel::NaiDiffusion45Full),
        GenerationPlanContext::default(),
    );
    let v5 = estimate_anlas_cost(
        &base_request(ImageModel::NaiDiffusion5Full),
        GenerationPlanContext::default(),
    );

    assert_eq!(v45.per_image_cost, 17);
    assert_eq!(v5.per_image_cost, 26);
    assert_eq!(v5.status, AnlasEstimateStatus::Available);
}

#[test]
fn opus_frees_only_the_first_image_of_a_request() {
    let request = GenerateImageRequest {
        n_samples: 4,
        ..base_request(ImageModel::NaiDiffusion45Full)
    };

    let free_tier = estimate_anlas_cost(&request, GenerationPlanContext::default());
    let opus = estimate_anlas_cost(&request, opus_context());

    assert!(!free_tier.free_first_image_applied);
    assert_eq!(free_tier.billable_samples, 4);
    assert_eq!(free_tier.total_cost, 68);

    assert!(opus.free_first_image_applied);
    assert_eq!(opus.priced_samples, 4);
    assert_eq!(opus.billable_samples, 3);
    assert_eq!(opus.total_cost, 51);
}

#[test]
fn single_image_opus_request_is_free() {
    let estimate = estimate_anlas_cost(
        &base_request(ImageModel::NaiDiffusion45Full),
        opus_context(),
    );

    assert!(estimate.is_free());
    assert_eq!(estimate.total_cost, 0);
}

#[test]
fn expired_subscription_loses_the_opus_allowance() {
    let context = GenerationPlanContext {
        subscription_active: false,
        ..opus_context()
    };

    let estimate = estimate_anlas_cost(&base_request(ImageModel::NaiDiffusion45Full), context);

    assert!(!estimate.free_first_image_applied);
    assert!(!estimate.is_free());
    assert_eq!(estimate.total_cost, 17);
}

#[test]
fn overdrawn_v5_allowance_loses_the_opus_allowance() {
    let context = GenerationPlanContext {
        v5_usage_is_negative: true,
        ..opus_context()
    };

    let v45 = estimate_anlas_cost(&base_request(ImageModel::NaiDiffusion45Full), context);
    let v5 = estimate_anlas_cost(&base_request(ImageModel::NaiDiffusion5Full), context);

    // Only V5 carries an Opus usage limit.
    assert!(v45.free_first_image_applied);
    assert!(!v5.free_first_image_applied);
    assert_eq!(v5.total_cost, 26);
}

#[test]
fn normalizes_out_of_range_canvas_before_estimating() {
    let request = GenerateImageRequest {
        size: ImageSize {
            width: 1792,
            height: 1664,
        },
        steps: 50,
        ..base_request(ImageModel::NaiDiffusion5Full)
    };

    let estimate = estimate_anlas_cost(&request, GenerationPlanContext::default());

    assert_eq!(estimate.per_image_cost, 123);
    assert_eq!(estimate.status, AnlasEstimateStatus::Available);
    assert!(!estimate.is_too_expensive());
    assert!(!estimate.is_free());
}

#[test]
fn charges_vibe_encoding_once_across_the_run() {
    let request = GenerateImageRequest {
        vibe_transfer: Some(VibeTransferConfig {
            references: vec![VibeReference {
                vibe_data_cache: "Y2FjaGVk".to_owned(),
                strength: 0.6,
            }],
            strength: 0.6,
        }),
        ..base_request(ImageModel::NaiDiffusion45Full)
    };
    let context = GenerationPlanContext {
        request_count: 3,
        pending_vibe_encode_count: 2,
        ..GenerationPlanContext::default()
    };

    let estimate = estimate_anlas_cost(&request, context);

    assert_eq!(estimate.per_request_cost, 17);
    assert_eq!(estimate.generation_cost, 51);
    // Four references are included, so a single active reference adds no overage.
    assert_eq!(estimate.vibe_reference_overage_cost, 0);
    assert_eq!(estimate.pending_encode_cost, 4);
    assert_eq!(estimate.total_cost, 55);
}

#[test]
fn charges_character_references_per_priced_image() {
    let request = GenerateImageRequest {
        n_samples: 2,
        character_references: Some(vec![CharacterReference {
            image: png_base64(),
            reference_type: CharacterReferenceType::Style,
            fidelity: 0.5,
            strength: 0.6,
        }]),
        ..base_request(ImageModel::NaiDiffusion45Full)
    };

    let estimate = estimate_anlas_cost(&request, GenerationPlanContext::default());

    assert_eq!(estimate.character_reference_cost, 10);
    assert_eq!(estimate.generation_cost, 34);
    assert_eq!(estimate.total_cost, 44);
}

#[test]
fn character_references_are_not_charged_on_models_without_support() {
    let request = GenerateImageRequest {
        n_samples: 2,
        character_references: Some(vec![CharacterReference {
            image: png_base64(),
            reference_type: CharacterReferenceType::Style,
            fidelity: 0.5,
            strength: 0.6,
        }]),
        ..base_request(ImageModel::NaiDiffusion5Full)
    };

    let estimate = estimate_anlas_cost(&request, GenerationPlanContext::default());

    assert_eq!(estimate.character_reference_cost, 0);
    assert_eq!(estimate.total_cost, 52);
}

#[test]
fn image_to_image_strength_scales_the_per_image_price() {
    let request = GenerateImageRequest {
        img2img: Some(Img2ImgRequest {
            image: png_base64(),
            strength: 0.7,
            noise: 0.0,
            inpaint: None,
        }),
        ..base_request(ImageModel::NaiDiffusion45Full)
    };

    let estimate = estimate_anlas_cost(&request, GenerationPlanContext::default());

    assert_eq!(estimate.per_image_cost, 12);
    assert_eq!(estimate.total_cost, 12);
}
