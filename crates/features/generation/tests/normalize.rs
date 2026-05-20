use nai_atelier_generation::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, ControlNetConfig,
    ControlNetInput, GenerateImageRequest, GenerationErrorKind, ImageModel, ImageSize,
    Img2ImgRequest, normalize_generate_request,
};

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn strict_mode_rejects_invalid_size_and_numeric_ranges() {
    let invalid_size = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        size: ImageSize {
            width: 65,
            height: 1024,
        },
        strict_mode: true,
        ..Default::default()
    };

    let error = normalize_generate_request(invalid_size).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::InvalidImageDimension);
    assert_eq!(error.field.as_deref(), Some("size.width"));

    let invalid_steps = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        steps: 0,
        strict_mode: true,
        ..Default::default()
    };

    let error = normalize_generate_request(invalid_steps).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::NumericOutOfRange);
    assert_eq!(error.field.as_deref(), Some("steps"));

    let invalid_scale = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        scale: 11.0,
        strict_mode: true,
        ..Default::default()
    };

    let error = normalize_generate_request(invalid_scale).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::NumericOutOfRange);
    assert_eq!(error.field.as_deref(), Some("scale"));
}

#[test]
fn rejects_non_finite_floats_even_without_strict_mode() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        scale: f32::NAN,
        strict_mode: false,
        ..Default::default()
    };

    let error = normalize_generate_request(request).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::NonFiniteNumber);
    assert_eq!(error.field.as_deref(), Some("scale"));
}

#[test]
fn strict_mode_reports_non_finite_character_positions() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        strict_mode: true,
        characters: Some(vec![Character {
            prompt: "alice".to_owned(),
            negative_prompt: None,
            position: CharacterPosition {
                x: f32::NAN,
                y: 0.5,
            },
            enabled: true,
        }]),
        ..Default::default()
    };

    let error = normalize_generate_request(request).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::NonFiniteNumber);
    assert_eq!(error.field.as_deref(), Some("characters[0].position.x"));
}

#[test]
fn non_strict_mode_clamps_and_snaps_request_values() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        size: ImageSize {
            width: 65,
            height: 1_599,
        },
        steps: 0,
        scale: 11.0,
        n_samples: 9,
        cfg_rescale: 2.0,
        i2i: Some(Img2ImgRequest {
            image: "image-payload-left-to-adapter".to_owned(),
            strength: 0.0,
            noise: 2.0,
            mask: None,
        }),
        controlnet: Some(ControlNetConfig {
            strength: 2.0,
            images: vec![ControlNetInput {
                vibe_data_cache: "encoded-vibe".to_owned(),
                info_extracted: 2.0,
                strength: -1.0,
            }],
        }),
        ..Default::default()
    };

    let normalized = normalize_generate_request(request).unwrap();

    assert_eq!(
        normalized.size,
        ImageSize {
            width: 64,
            height: 1_600,
        }
    );
    assert_eq!(normalized.steps, 1);
    assert_f32_eq(normalized.scale, 10.0);
    assert_eq!(normalized.n_samples, 4);
    assert_f32_eq(normalized.cfg_rescale, 1.0);
    let i2i = normalized.i2i.unwrap();
    assert_f32_eq(i2i.strength, 0.01);
    assert_f32_eq(i2i.noise, 0.99);
    let controlnet = normalized.controlnet.unwrap();
    assert_f32_eq(controlnet.strength, 1.0);
    assert_f32_eq(controlnet.images[0].info_extracted, 1.0);
    assert_f32_eq(controlnet.images[0].strength, 0.0);
}

#[test]
fn model_capability_gates_reject_invalid_feature_combinations() {
    let characters_on_v3 = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        model: ImageModel::NaiDiffusion3,
        characters: Some(vec![Character {
            prompt: "alice".to_owned(),
            negative_prompt: None,
            position: CharacterPosition::default(),
            enabled: true,
        }]),
        ..Default::default()
    };

    let error = normalize_generate_request(characters_on_v3).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::UnsupportedModelFeature);
    assert_eq!(error.field.as_deref(), Some("characters"));

    let reference_on_v4 = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        model: ImageModel::NaiDiffusion4Full,
        character_references: Some(vec![CharacterReference {
            image: "image-payload-left-to-adapter".to_owned(),
            reference_type: CharacterReferenceType::Character,
            fidelity: 0.5,
            strength: 0.5,
        }]),
        ..Default::default()
    };

    let error = normalize_generate_request(reference_on_v4).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::UnsupportedModelFeature);
    assert_eq!(error.field.as_deref(), Some("character_references"));

    let controlnet_with_reference = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        controlnet: Some(ControlNetConfig {
            strength: 0.5,
            images: vec![ControlNetInput {
                vibe_data_cache: "encoded-vibe".to_owned(),
                info_extracted: 0.5,
                strength: 0.5,
            }],
        }),
        character_references: Some(vec![CharacterReference {
            image: "image-payload-left-to-adapter".to_owned(),
            reference_type: CharacterReferenceType::Style,
            fidelity: 0.5,
            strength: 0.5,
        }]),
        ..Default::default()
    };

    let error = normalize_generate_request(controlnet_with_reference).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::UnsupportedFieldCombination);
    assert_eq!(
        error.field.as_deref(),
        Some("controlnet+character_references")
    );
}
