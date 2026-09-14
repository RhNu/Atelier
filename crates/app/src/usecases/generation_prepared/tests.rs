use super::{PreparedPromptTrace, PreparedTextGeneration};
use atelier_generation::{GenerateImageRequest, QualityPreset, UcPreset};
use atelier_kernel::GenerationWorkRequest;
use atelier_prompt_resources::{CompiledPrompt, PromptTrace};

#[test]
fn prepared_work_preserves_literal_compiled_text_and_normalizes_settings() {
    let literal = "a sign reading $chunk(example)";
    let draft =
        crate::mapping::generation_draft_to_dto(&crate::usecases::generation_draft::default_draft(
            &atelier_settings::WorkspaceSettings::default(),
        ));
    let request = atelier_app_api::generation::GenerateImageRequestDto {
        main_preset_id: None,
        prompt: literal.to_owned(),
        model: draft.model,
        size: draft.size,
        furry_mode: false,
        negative_prompt: None,
        quality: draft.quality,
        transparent_background: false,
        uc_preset: draft.uc_preset,
        steps: draft.steps,
        scale: draft.scale,
        sampler: draft.sampler,
        noise_schedule: draft.noise_schedule,
        seed: draft.seed,
        n_samples: draft.n_samples,
        cfg_rescale: draft.cfg_rescale,
        variety_boost: false,
        strict_mode: false,
        img2img: None,
        vibe_transfer: None,
        character_references: None,
        characters: None,
        use_coords: None,
        image_format: None,
    };
    let domain = GenerateImageRequest {
        prompt: literal.to_owned(),
        steps: 99,
        strict_mode: false,
        quality: QualityPreset::None,
        uc_preset: UcPreset::None,
        ..GenerateImageRequest::default()
    };
    let prompt = CompiledPrompt {
        expanded_prompt: literal.to_owned(),
        trace: PromptTrace {
            raw_prompt: "input".into(),
            expanded_prompt: literal.to_owned(),
            function_calls: Vec::new(),
        },
    };
    let prepared = PreparedTextGeneration::from_compiled(
        request,
        domain,
        prompt,
        PreparedPromptTrace::default(),
        false,
    )
    .unwrap();
    let GenerationWorkRequest::Image(work) = prepared.work() else {
        panic!("expected image request")
    };
    assert_eq!(work.prompt, literal);
    assert_eq!(prepared.request().steps, 50);
    assert_eq!(work.steps, 50);
    assert_eq!(prepared.resolved_prompts().unwrap()["prompt"], literal);
}
