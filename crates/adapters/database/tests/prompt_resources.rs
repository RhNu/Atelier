use atelier_adapter_database::{DatabaseConnection, DatabasePromptResourceRepository};
use atelier_prompt_resources::{
    PromptChunkKey, PromptChunkService, PromptResourceErrorKind, PromptResourceReader,
    UpsertPromptChunkRequest,
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
