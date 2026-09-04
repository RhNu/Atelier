use atelier_resource_catalog::{ResourceId, ResourceRef};

use super::*;

#[test]
fn complete_generation_draft_is_valid() {
    sample_draft().validate().unwrap();
}

#[test]
fn generation_draft_rejects_invalid_numeric_and_position_values() {
    let mut draft = sample_draft();
    draft.size.width = 800;
    assert_eq!(
        draft.validate().unwrap_err().field.as_deref(),
        Some("size.width")
    );

    let mut draft = sample_draft();
    draft.prompt_states[0].characters[0].position.x = 1.1;
    assert_eq!(
        draft.validate().unwrap_err().field.as_deref(),
        Some("prompt_states[0].characters[0].position.x")
    );
}

#[test]
fn generation_draft_rejects_vibe_and_precise_reference_conflict() {
    let mut draft = sample_draft();
    draft.vibe.enabled = true;
    assert_eq!(draft.validate().unwrap_err().field.as_deref(), Some("vibe"));
}

fn sample_draft() -> GenerationDraftSnapshot {
    GenerationDraftSnapshot {
        model: ImageModel::NaiDiffusion45Full,
        prompt_states: vec![GenerationDraftPromptState {
            model: ImageModel::NaiDiffusion45Full,
            main_preset_id: Some("main-preset".to_owned()),
            prompt: "1girl, cinematic lighting".to_owned(),
            negative_prompt: "low quality".to_owned(),
            characters: vec![GenerationDraftCharacter {
                id: "character-slot".to_owned(),
                preset_id: Some("character-preset".to_owned()),
                prompt: "red hair".to_owned(),
                negative_prompt: "hat".to_owned(),
                enabled: true,
                position: CharacterPosition { x: 0.25, y: 0.75 },
            }],
            character_position_mode: GenerationDraftCharacterPositionMode::Manual,
        }],
        size: ImageSize {
            width: 832,
            height: 1216,
        },
        quality: QualityPreset::Standard,
        transparent_background: false,
        uc_preset: UcPreset::Light,
        steps: 28,
        scale: 5.5,
        sampler: Sampler::KDpmpp2m,
        noise_schedule: NoiseSchedule::Exponential,
        seed_mode: GenerationDraftSeedMode::Fixed,
        seed: 42,
        n_samples: 2,
        request_count: 3,
        cfg_rescale: 0.25,
        variety_boost: true,
        image_format: Some(ImageFormat::Webp),
        strict_mode: true,
        stream_enabled: false,
        i2i: Some(GenerationDraftI2i {
            image: resource("resource:i2i"),
            inpaint: Some(GenerationDraftInpaintSession {
                region_to_replace: resource("resource:mask"),
                display: GenerationDraftMaskDisplay::default(),
            }),
            strength: 0.6,
            noise: 0.2,
        }),
        vibe: GenerationDraftVibe {
            enabled: false,
            strength: 0.8,
            slots: vec![GenerationDraftVibeSlot {
                id: "vibe-slot".to_owned(),
                encoding: resource("resource:vibe-encoding"),
                vibe_id: Some("vibe-document".to_owned()),
                information_extracted: 0.7,
                strength: 0.9,
                display_name: "Warm film".to_owned(),
                source_image: Some(resource("resource:vibe-source")),
                source_sha256: Some("abc123".to_owned()),
                model: ImageModel::NaiDiffusion45Full,
            }],
        },
        precise_references: vec![GenerationDraftPreciseReference {
            id: "reference-slot".to_owned(),
            image: resource("resource:reference"),
            reference_type: CharacterReferenceType::Character,
            fidelity: 0.5,
            strength: 0.6,
            display_name: "Hero".to_owned(),
        }],
    }
}

fn resource(id: &str) -> ResourceRef {
    ResourceRef::base(ResourceId::new(id))
}
