use super::*;
use serde_json::json;

fn draft() -> GenerationDraftDto {
    let mut draft =
        crate::mapping::generation_draft_to_dto(&crate::usecases::generation_draft::default_draft(
            &atelier_settings::WorkspaceSettings::default(),
        ));
    let state = &mut draft.prompt_states[0];
    state.prompt = "forest, sunrise".to_owned();
    state.characters = vec![GenerationDraftCharacterDto {
        id: "hero".to_owned(),
        preset_id: Some("outfit".to_owned()),
        prompt: "red hair".to_owned(),
        negative_prompt: "hat".to_owned(),
        enabled: false,
        position: CharacterPositionDto { x: 0.2, y: 0.8 },
    }];
    draft
}

fn edit(value: serde_json::Value) -> DraftEdit {
    serde_json::from_value(value).unwrap()
}

#[test]
fn exact_character_edit_preserves_other_fields() {
    let original = draft();
    let result = edit(json!({"operations":[{"op":"edit_text","target":{"character_id":"hero","field":"prompt"},"edit":{"mode":"replace","old_text":"red hair","new_text":"silver hair"}}]})).apply(&original).unwrap();
    let mut expected = original.prompt_states[0].characters[0].clone();
    expected.prompt = "silver hair".to_owned();
    assert_eq!(result.prompt_states[0].characters[0], expected);
}

#[test]
fn failed_operation_discards_the_entire_batch() {
    let original = draft();
    let result = edit(json!({"operations":[
        {"op":"edit_text","target":{"field":"prompt"},"edit":{"mode":"set","text":"ocean"}},
        {"op":"edit_text","target":{"field":"prompt"},"edit":{"mode":"replace","old_text":"missing","new_text":"sky"}}
    ]})).apply(&original);
    assert!(result.unwrap_err().message.contains("text_not_found"));
    assert_eq!(original.prompt_states[0].prompt, "forest, sunrise");
}

#[test]
fn operations_are_sequential_and_support_explicit_deselection() {
    let result = edit(json!({"operations":[
        {"op":"copy_character","id":"hero","new_id":"other"},
        {"op":"select_preset","character_id":"other","preset_id":null},
        {"op":"update_character","id":"other","patch":{"enabled":true}},
        {"op":"reorder_characters","ids":["other","hero"]}
    ]}))
    .apply(&draft())
    .unwrap();
    assert_eq!(result.prompt_states[0].characters[0].id, "other");
    assert_eq!(result.prompt_states[0].characters[0].preset_id, None);
    assert!(result.prompt_states[0].characters[0].enabled);
    assert_eq!(result.prompt_states[0].characters[0].negative_prompt, "hat");
    assert!(!result.prompt_states[0].characters[1].enabled);
}

#[test]
fn reordering_cannot_duplicate_or_lose_characters() {
    assert!(
        edit(json!({"operations":[{"op":"reorder_characters","ids":[]}]}))
            .apply(&draft())
            .is_err()
    );
    assert!(
        edit(json!({"operations":[{"op":"copy_character","id":"hero","new_id":"hero"}]}))
            .apply(&draft())
            .is_err()
    );
}

#[test]
fn model_switch_restores_its_prompt_buffer() {
    let original = draft();
    let result = edit(json!({"operations":[
        {"op":"select_model","model":"nai-diffusion-3"},
        {"op":"edit_text","target":{"field":"prompt"},"edit":{"mode":"set","text":"ocean"}},
        {"op":"select_model","model":original.model}
    ]}))
    .apply(&original)
    .unwrap();
    assert_eq!(result.prompt_states[0].prompt, "forest, sunrise");
    assert_eq!(result.prompt_states[1].prompt, "ocean");
}

#[test]
fn settings_patch_preserves_unspecified_values_and_validates_ranges() {
    let original = draft();
    let result = edit(json!({"operations":[{"op":"set_parameters","patch":{"steps":17,"seed_mode":"fixed","seed":42}}]})).apply(&original).unwrap();
    assert_eq!(result.steps, 17);
    assert_eq!(result.seed_mode, GenerationDraftSeedModeDto::Fixed);
    assert_eq!(result.seed, 42);
    assert_eq!(result.sampler, original.sampler);
    assert!(
        edit(json!({"operations":[{"op":"set_parameters","patch":{"steps":0}}]}))
            .apply(&original)
            .is_err()
    );
}
