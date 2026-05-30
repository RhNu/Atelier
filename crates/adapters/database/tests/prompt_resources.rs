use atelier_adapter_database::{DatabaseConnection, DatabasePromptResourceRepository};
use atelier_prompt_resources::{
    PromptChunkKey, PromptChunkService, PromptPresetKind, PromptPresetService,
    PromptResourceErrorKind, PromptResourceReader, UpsertPromptChunkRequest,
    UpsertPromptPresetRequest,
};
use futures_executor::block_on;

#[test]
fn prompt_repository_crud_rewrites_references_and_blocks_referenced_delete() {
    block_on(async {
        let repository =
            DatabasePromptResourceRepository::new(DatabaseConnection::open_memory().unwrap());
        let service = PromptChunkService::new(repository.clone());

        let hero = service
            .upsert_chunk(request(None, "hero", "1girl"))
            .await
            .unwrap();
        let scene = service
            .upsert_chunk(request(None, "scene", "@chunk(hero), forest"))
            .await
            .unwrap();

        let renamed = service
            .upsert_chunk(request(Some(hero.id.clone()), "main_hero", "1girl, solo"))
            .await
            .unwrap();

        assert_eq!(renamed.key.as_str(), "main_hero");
        assert_eq!(
            service
                .get_chunk_by_id(&scene.id)
                .await
                .unwrap()
                .unwrap()
                .content,
            "@chunk(main_hero), forest"
        );
        let delete_error = service.delete_chunk(&renamed.id).await.unwrap_err();
        assert_eq!(delete_error.kind(), PromptResourceErrorKind::Conflict);

        service.delete_chunk(&scene.id).await.unwrap();
        assert!(service.delete_chunk(&renamed.id).await.unwrap().deleted);
        assert!(repository.list_chunks().await.unwrap().is_empty());
    });
}

#[test]
fn prompt_repository_persists_presets_and_rewrites_preset_chunk_references() {
    block_on(async {
        let repository =
            DatabasePromptResourceRepository::new(DatabaseConnection::open_memory().unwrap());
        let chunk_service = PromptChunkService::new(repository.clone());
        let preset_service = PromptPresetService::new(repository.clone());

        let chunk = chunk_service
            .upsert_chunk(request(None, "old-key", "detail"))
            .await
            .unwrap();
        let mut main_preset_request = preset_request(None, PromptPresetKind::Main, "Main", 5);
        main_preset_request.before = "@chunk(old-key)".to_owned();
        let preset = preset_service
            .upsert_preset(main_preset_request)
            .await
            .unwrap();
        preset_service
            .upsert_preset(preset_request(
                None,
                PromptPresetKind::Character,
                "Character",
                0,
            ))
            .await
            .unwrap();

        let renamed = chunk_service
            .upsert_chunk(request(Some(chunk.id), "new-key", "detail"))
            .await
            .unwrap();

        let rewritten = preset_service
            .get_preset_by_id(&preset.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rewritten.before, "@chunk(new-key)");
        let delete_error = chunk_service.delete_chunk(&renamed.id).await.unwrap_err();
        assert_eq!(delete_error.kind(), PromptResourceErrorKind::Conflict);
        assert_eq!(
            preset_service
                .list_presets(Some(PromptPresetKind::Main), false)
                .await
                .unwrap()
                .len(),
            1
        );
    });
}

#[test]
fn prompt_repository_rejects_duplicate_keys() {
    block_on(async {
        let repository =
            DatabasePromptResourceRepository::new(DatabaseConnection::open_memory().unwrap());
        let service = PromptChunkService::new(repository);

        service
            .upsert_chunk(request(None, "hero", "1girl"))
            .await
            .unwrap();
        let error = service
            .upsert_chunk(request(None, "hero", "2girls"))
            .await
            .unwrap_err();

        assert_eq!(error.kind(), PromptResourceErrorKind::Conflict);
    });
}

fn request(
    chunk_id: Option<atelier_prompt_resources::PromptChunkId>,
    key: &str,
    content: &str,
) -> UpsertPromptChunkRequest {
    UpsertPromptChunkRequest {
        chunk_id,
        key: PromptChunkKey::parse(key).unwrap(),
        content: content.to_owned(),
        category: None,
        description: None,
        preview_thumb: None,
    }
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
