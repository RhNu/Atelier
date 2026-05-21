use atelier_generation::{
    Character, CharacterPosition, GenerateImageRequest, GenerationErrorKind, GenerationOutputMode,
    GenerationPlanContext, SeedMode, plan_generation_request,
};

#[test]
fn request_plan_rejects_empty_prompt() {
    let request = GenerateImageRequest {
        prompt: "  ".to_owned(),
        ..Default::default()
    };

    let error = plan_generation_request(request, GenerationPlanContext::default()).unwrap_err();

    assert_eq!(error.kind, GenerationErrorKind::EmptyField);
    assert_eq!(error.field.as_deref(), Some("prompt"));
}

#[test]
fn request_plan_records_auto_seed_without_resolving_it() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        seed: 0,
        ..Default::default()
    };

    let plan = plan_generation_request(request, GenerationPlanContext::default()).unwrap();

    assert_eq!(plan.seed_mode, SeedMode::Auto);
    assert_eq!(plan.normalized_request.seed, 0);
    assert_eq!(plan.output_mode, GenerationOutputMode::Image);
}

#[test]
fn request_plan_keeps_fixed_seed() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        seed: 42,
        ..Default::default()
    };

    let plan = plan_generation_request(request, GenerationPlanContext::default()).unwrap();

    assert_eq!(plan.seed_mode, SeedMode::Fixed(42));
    assert_eq!(plan.normalized_request.seed, 42);
}

#[test]
fn explicit_use_coords_wins_for_enabled_characters() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        use_coords: Some(true),
        characters: Some(vec![Character {
            prompt: "alice".to_owned(),
            negative_prompt: None,
            position: CharacterPosition::default(),
            enabled: true,
        }]),
        ..Default::default()
    };

    let plan = plan_generation_request(request, GenerationPlanContext::default()).unwrap();

    assert!(plan.resolved_use_coords);
}

#[test]
fn use_coords_is_derived_from_enabled_non_center_character() {
    let request = GenerateImageRequest {
        prompt: "1girl".to_owned(),
        characters: Some(vec![
            Character {
                prompt: "disabled".to_owned(),
                negative_prompt: None,
                position: CharacterPosition { x: 0.1, y: 0.9 },
                enabled: false,
            },
            Character {
                prompt: "alice".to_owned(),
                negative_prompt: None,
                position: CharacterPosition { x: 0.7, y: 0.5 },
                enabled: true,
            },
        ]),
        ..Default::default()
    };

    let plan = plan_generation_request(request, GenerationPlanContext::default()).unwrap();

    assert!(plan.resolved_use_coords);
}
