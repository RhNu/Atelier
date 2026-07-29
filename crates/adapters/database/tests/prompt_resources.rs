use atelier_adapter_database::{DatabaseConnection, DatabasePromptResourceRepository};
use atelier_prompt_resources::{
    PromptChunkKey, PromptChunkService, PromptPresetBehavior, PromptPresetKind,
    PromptPresetService, PromptResourceErrorKind, PromptResourceReader, UpsertPromptChunkRequest,
    UpsertPromptPresetRequest,
};
use futures_executor::block_on;
use rusqlite::Connection;

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
            .upsert_chunk(request(None, "scene", "$chunk(hero), forest"))
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
            "$chunk(main_hero), forest"
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
        main_preset_request.prompt_behavior = surround("$chunk(old-key)", "");
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
        assert_eq!(rewritten.prompt_behavior, surround("$chunk(new-key)", ""));
        let delete_error = chunk_service.delete_chunk(&renamed.id).await.unwrap_err();
        assert_eq!(delete_error.kind(), PromptResourceErrorKind::Conflict);
        assert_eq!(
            preset_service
                .list_presets(Some(PromptPresetKind::Main))
                .await
                .unwrap()
                .len(),
            1
        );
    });
}

#[test]
fn prompt_repository_preserves_an_empty_replace_behavior() {
    block_on(async {
        let repository =
            DatabasePromptResourceRepository::new(DatabaseConnection::open_memory().unwrap());
        let service = PromptPresetService::new(repository);
        let mut request = preset_request(None, PromptPresetKind::Main, "Empty replace", 0);
        request.prompt_behavior = replace("");

        let saved = service.upsert_preset(request).await.unwrap();
        let reloaded = service.get_preset_by_id(&saved.id).await.unwrap().unwrap();

        assert_eq!(reloaded.prompt_behavior, replace(""));
    });
}

#[test]
fn prompt_behavior_migration_infers_modes_for_legacy_presets() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            r"
            CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_ms INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO schema_migrations(version) VALUES (1), (2), (3), (4), (5), (6), (7);
            CREATE TABLE prompt_presets (
                preset_id TEXT PRIMARY KEY, preset_kind TEXT NOT NULL, name TEXT NOT NULL,
                category TEXT, description TEXT, sort_order INTEGER NOT NULL,
                enabled INTEGER NOT NULL, before_text TEXT NOT NULL, after_text TEXT NOT NULL,
                replace_text TEXT NOT NULL, uc_before_text TEXT NOT NULL,
                uc_after_text TEXT NOT NULL, uc_replace_text TEXT NOT NULL,
                quality_override TEXT, uc_preset_override TEXT, preview_resource_id TEXT,
                preview_variant_id TEXT, created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            INSERT INTO prompt_presets VALUES (
                'legacy', 'main', 'Legacy', NULL, NULL, 0, 0,
                'ignored', 'ignored', 'replacement', 'uc-before', 'uc-after', '',
                NULL, NULL, NULL, NULL, 1, 1
            );
            ",
        )
        .unwrap();
    drop(connection);

    let repository =
        DatabasePromptResourceRepository::new(DatabaseConnection::open(&path).unwrap());
    let preset = block_on(repository.list_presets(None)).unwrap().remove(0);

    assert_eq!(preset.prompt_behavior, replace("replacement"));
    assert_eq!(preset.uc_behavior, surround("uc-before", "uc-after"));
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
        prompt_behavior: surround("", ""),
        uc_behavior: surround("", ""),
        quality_override: None,
        uc_preset_override: None,
        preview_thumb: None,
    }
}

fn surround(before: &str, after: &str) -> PromptPresetBehavior {
    PromptPresetBehavior::Surround {
        before: before.to_owned(),
        after: after.to_owned(),
    }
}

fn replace(text: &str) -> PromptPresetBehavior {
    PromptPresetBehavior::Replace {
        text: text.to_owned(),
    }
}
