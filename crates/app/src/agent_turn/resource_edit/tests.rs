use super::{ResourceDocument, ResourceOperation};
use atelier_app_api::{
    generation::{ImageModelDto, QualityPresetDto},
    prompt::{PromptPresetBehaviorDto, PromptPresetDto, PromptPresetKindDto},
};

fn preset() -> ResourceDocument {
    ResourceDocument::Preset(PromptPresetDto {
        preset_id: "portrait".into(),
        kind: PromptPresetKindDto::Main,
        path: "portrait".into(),
        folder_id: None,
        identifier: "portrait".into(),
        display_name: "Portrait".into(),
        aliases: vec!["face".into()],
        description: Some("description".into()),
        prompt_behavior: PromptPresetBehaviorDto::Surround {
            before: "red hair, ".into(),
            after: ", soft light".into(),
        },
        uc_behavior: PromptPresetBehaviorDto::Replace {
            text: "blurry".into(),
        },
        quality_override: Some(QualityPresetDto::Light),
        uc_preset_override: None,
        preview: None,
        models: vec![ImageModelDto::NaiDiffusion4Curated],
        created_at_ms: 1,
        updated_at_ms: 1,
    })
}

fn operations(json: &str) -> Vec<ResourceOperation> {
    serde_json::from_str(json).unwrap()
}

#[test]
fn precise_text_edit_preserves_other_preset_fields() {
    let before = preset();
    let after = before.edit(&operations(r#"[{"op":"edit_text","field":"prompt_before","edit":{"mode":"replace","old_text":"red","new_text":"blue"}}]"#)).unwrap();
    let ResourceDocument::Preset(mut expected) = before else {
        unreachable!()
    };
    expected.prompt_behavior = PromptPresetBehaviorDto::Surround {
        before: "blue hair, ".into(),
        after: ", soft light".into(),
    };
    assert_eq!(after, ResourceDocument::Preset(expected));
}

#[test]
fn failing_later_operation_leaves_original_untouched() {
    let before = preset();
    let original = before.clone();
    assert!(before.edit(&operations(r#"[{"op":"set_metadata","patch":{"display_name":"Changed"}},{"op":"edit_text","field":"prompt_text","edit":{"mode":"set","text":"wrong mode"}}]"#)).is_err());
    assert_eq!(before, original);
}

#[test]
fn explicit_null_clears_override_and_description() {
    let ResourceDocument::Preset(after) = preset().edit(&operations(r#"[{"op":"set_metadata","patch":{"description":null}},{"op":"set_overrides","patch":{"quality":null}}]"#)).unwrap() else { unreachable!() };
    assert_eq!(after.description, None);
    assert_eq!(after.quality_override, None);
    assert_eq!(after.aliases, vec!["face"]);
}

#[test]
fn behavior_change_and_replacement_apply_in_order() {
    let ResourceDocument::Preset(after) = preset().edit(&operations(r#"[{"op":"set_behavior","field":"negative_prompt","behavior":{"mode":"surround","before":"low quality, ","after":""}},{"op":"edit_text","field":"uc_before","edit":{"mode":"replace","old_text":"low","new_text":"bad"}}]"#)).unwrap() else { unreachable!() };
    assert_eq!(
        after.uc_behavior,
        PromptPresetBehaviorDto::Surround {
            before: "bad quality, ".into(),
            after: String::new()
        }
    );
}

#[test]
fn chunk_rename_advances_draft_version_and_rejects_the_old_edit() {
    use atelier_generation::{VersionedGenerationDraft, prepare_generation_draft_save};
    use atelier_prompt_resources::{PromptChunkKey, rewrite_draft_chunk_references};
    let mut before = crate::usecases::generation_draft::default_draft(
        &atelier_settings::WorkspaceSettings::default(),
    );
    before.prompt_states[0].prompt = "portrait, $chunk(old)".into();
    let current = VersionedGenerationDraft {
        revision: 8,
        snapshot: before.clone(),
    };
    let mut renamed = before.clone();
    rewrite_draft_chunk_references(
        &mut renamed,
        &[(
            PromptChunkKey::parse("old").unwrap(),
            PromptChunkKey::parse("new").unwrap(),
        )],
    );
    assert_eq!(renamed.prompt_states[0].prompt, "portrait, $chunk(new)");
    assert_eq!(renamed.model, before.model);
    let saved = prepare_generation_draft_save(Some(&current), 8, 8, &renamed).unwrap();
    assert_eq!(saved.revision, 9);
    assert!(prepare_generation_draft_save(Some(&saved), 8, 9, &before).is_err());
}
