mod support;

use atelier_prompt_resources::{
    CompileCharacterPromptRequest, CompileGenerationPromptRequest, PromptChunkKey,
    PromptChunkService, PromptCompiler, PromptPresetKind, PromptPresetService,
    PromptResourceErrorKind, UpsertPromptChunkRequest, UpsertPromptPresetRequest,
};
use futures_executor::block_on;
use support::MemoryPromptResourceRepository;

#[test]
fn preset_service_manages_main_and_character_presets() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptPresetService::new(repository);

        let late = service
            .upsert_preset(preset_request(None, PromptPresetKind::Main, "Late", 10))
            .await
            .unwrap();
        let early = service
            .upsert_preset(preset_request(None, PromptPresetKind::Main, "Early", -10))
            .await
            .unwrap();
        service
            .upsert_preset(preset_request(None, PromptPresetKind::Character, "Hero", 0))
            .await
            .unwrap();

        let main = service
            .list_presets(Some(PromptPresetKind::Main), false)
            .await
            .unwrap();
        assert_eq!(
            main.iter()
                .map(|preset| preset.id.clone())
                .collect::<Vec<_>>(),
            vec![early.id, late.id]
        );
        assert_eq!(
            service
                .list_presets(Some(PromptPresetKind::Character), false)
                .await
                .unwrap()
                .len(),
            1
        );
    });
}

#[test]
fn character_presets_cannot_define_generation_overrides() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptPresetService::new(repository);
        let mut request = preset_request(None, PromptPresetKind::Character, "Invalid", 0);
        request.quality_override = Some("native".to_owned());

        let error = service.upsert_preset(request).await.unwrap_err();

        assert_eq!(error.kind(), PromptResourceErrorKind::InvalidRequest);
    });
}

#[test]
fn compiler_applies_presets_and_expands_chunks_inside_preset_fields() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let chunk_service = PromptChunkService::new(repository.clone());
        chunk_service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: None,
                key: PromptChunkKey::parse("lighting").unwrap(),
                content: "cinematic lighting".to_owned(),
                category: None,
                description: None,
                preview_thumb: None,
            })
            .await
            .unwrap();

        let preset_service = PromptPresetService::new(repository.clone());
        let mut main_request = preset_request(None, PromptPresetKind::Main, "Main", 0);
        main_request.before = "@chunk(lighting)".to_owned();
        main_request.after = "sharp focus".to_owned();
        main_request.uc_before = "bad anatomy".to_owned();
        main_request.quality_override = Some("qualityTagsV4".to_owned());
        main_request.uc_preset_override = Some("heavy".to_owned());
        let main = preset_service.upsert_preset(main_request).await.unwrap();

        let mut character_request = preset_request(None, PromptPresetKind::Character, "Hero", 0);
        character_request.before = "red hair".to_owned();
        character_request.uc_after = "extra arms".to_owned();
        let character = preset_service
            .upsert_preset(character_request)
            .await
            .unwrap();

        let compiler = PromptCompiler::new(repository);
        let result = compiler
            .compile_generation_prompt(CompileGenerationPromptRequest {
                main_preset_id: Some(main.id.clone()),
                prompt: "1girl".to_owned(),
                negative_prompt: "lowres".to_owned(),
                characters: vec![CompileCharacterPromptRequest {
                    character_index: 0,
                    preset_id: Some(character.id.clone()),
                    prompt: "solo".to_owned(),
                    negative_prompt: "worst quality".to_owned(),
                }],
                max_depth: 8,
            })
            .await
            .unwrap();

        assert_eq!(
            result.prompt,
            "cinematic lighting, 1girl, sharp focus".to_owned()
        );
        assert_eq!(result.negative_prompt, "bad anatomy, lowres".to_owned());
        assert_eq!(result.quality_override, Some("qualityTagsV4".to_owned()));
        assert_eq!(result.uc_preset_override, Some("heavy".to_owned()));
        assert_eq!(result.characters[0].prompt, "red hair, solo".to_owned());
        assert_eq!(
            result.characters[0].negative_prompt,
            "worst quality, extra arms".to_owned()
        );
        assert_eq!(result.trace.used_presets.len(), 2);
    });
}

#[test]
fn chunk_rename_rewrites_preset_fields() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let chunk_service = PromptChunkService::new(repository.clone());
        let chunk = chunk_service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: None,
                key: PromptChunkKey::parse("old-key").unwrap(),
                content: "detail".to_owned(),
                category: None,
                description: None,
                preview_thumb: None,
            })
            .await
            .unwrap();
        let preset_service = PromptPresetService::new(repository.clone());
        let mut request = preset_request(None, PromptPresetKind::Main, "Uses chunk", 0);
        request.before = "@chunk(old-key)".to_owned();
        let preset = preset_service.upsert_preset(request).await.unwrap();

        chunk_service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: Some(chunk.id),
                key: PromptChunkKey::parse("new-key").unwrap(),
                content: "detail".to_owned(),
                category: None,
                description: None,
                preview_thumb: None,
            })
            .await
            .unwrap();

        let rewritten = preset_service
            .get_preset_by_id(&preset.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rewritten.before, "@chunk(new-key)");
    });
}

#[test]
fn preset_references_block_chunk_delete() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let chunk_service = PromptChunkService::new(repository.clone());
        let preset_service = PromptPresetService::new(repository);
        let chunk = chunk_service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: None,
                key: PromptChunkKey::parse("lighting").unwrap(),
                content: "cinematic lighting".to_owned(),
                category: None,
                description: None,
                preview_thumb: None,
            })
            .await
            .unwrap();
        let mut request = preset_request(None, PromptPresetKind::Main, "Main", 0);
        request.before = "@chunk(lighting)".to_owned();
        preset_service.upsert_preset(request).await.unwrap();

        let error = chunk_service.delete_chunk(&chunk.id).await.unwrap_err();

        assert_eq!(error.kind(), PromptResourceErrorKind::Conflict);
    });
}

fn preset_request(
    preset_id: Option<atelier_prompt_resources::PromptPresetId>,
    kind: PromptPresetKind,
    name: &str,
    order: i32,
) -> UpsertPromptPresetRequest {
    UpsertPromptPresetRequest {
        preset_id,
        kind,
        name: name.to_owned(),
        category: None,
        description: None,
        order,
        enabled: true,
        before: String::new(),
        after: String::new(),
        replace: String::new(),
        uc_before: String::new(),
        uc_after: String::new(),
        uc_replace: String::new(),
        quality_override: None,
        uc_preset_override: None,
        preview_thumb: None,
    }
}
