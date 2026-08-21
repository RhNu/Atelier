mod support;

use atelier_generation::ImageModel;
use atelier_prompt_resources::{
    DeletePromptChunkResult, PromptChunkKey, PromptChunkService, PromptResourceErrorKind,
    UpsertPromptChunkRequest,
};
use futures_executor::block_on;
use support::MemoryPromptResourceRepository;

#[test]
fn chunk_service_manages_chunks_and_validates_keys() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptChunkService::new(repository.clone());

        let chunk = service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: None,
                key: PromptChunkKey::parse("光照-1").unwrap(),
                content: "cinematic lighting".to_owned(),
                category: Some("style".to_owned()),
                description: Some("lighting preset".to_owned()),
                preview_thumb: None,
                models: vec![ImageModel::NaiDiffusion45Full],
            })
            .await
            .unwrap();

        assert_eq!(chunk.key.as_str(), "光照-1");
        assert_eq!(
            service.get_chunk_by_key(&chunk.key).await.unwrap(),
            Some(chunk)
        );
        assert_eq!(service.list_chunks(None).await.unwrap().len(), 1);
        assert!(PromptChunkKey::parse("1bad").is_err());
    });
}

#[test]
fn chunk_keys_are_case_sensitive() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptChunkService::new(repository);

        service
            .upsert_chunk(request(None, "Face", "upper"))
            .await
            .unwrap();
        service
            .upsert_chunk(request(None, "face", "lower"))
            .await
            .unwrap();

        assert_eq!(
            service
                .get_chunk_by_key(&PromptChunkKey::parse("Face").unwrap())
                .await
                .unwrap()
                .unwrap()
                .content,
            "upper"
        );
        assert_eq!(service.list_chunks(None).await.unwrap().len(), 2);
    });
}

#[test]
fn chunk_rename_rewrites_existing_chunk_references() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptChunkService::new(repository);
        let base = service
            .upsert_chunk(request(None, "old-key", "base"))
            .await
            .unwrap();
        let dependent = service
            .upsert_chunk(request(None, "scene", "1girl, $chunk(old-key)"))
            .await
            .unwrap();

        service
            .upsert_chunk(request(Some(base.id.clone()), "new-key", "base"))
            .await
            .unwrap();

        let rewritten = service
            .get_chunk_by_id(&dependent.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(rewritten.content, "1girl, $chunk(new-key)");
        let renamed = service.get_chunk_by_id(&base.id).await.unwrap().unwrap();
        assert_eq!(renamed.content, "base");
    });
}

#[test]
fn chunk_delete_is_blocked_when_referenced() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptChunkService::new(repository);
        let base = service
            .upsert_chunk(request(None, "base", "detail"))
            .await
            .unwrap();
        let dependent = service
            .upsert_chunk(request(None, "scene", "$chunk(base)"))
            .await
            .unwrap();

        let error = service.delete_chunk(&base.id).await.unwrap_err();

        assert_eq!(error.kind(), PromptResourceErrorKind::Conflict);
        assert_eq!(error.references(), &[dependent.key]);
    });
}

#[test]
fn unreferenced_chunk_can_be_deleted() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptChunkService::new(repository);
        let chunk = service
            .upsert_chunk(request(None, "unused", "detail"))
            .await
            .unwrap();

        assert_eq!(
            service.delete_chunk(&chunk.id).await.unwrap(),
            DeletePromptChunkResult { deleted: true }
        );
        assert!(service.get_chunk_by_id(&chunk.id).await.unwrap().is_none());
    });
}

#[test]
fn model_bindings_filter_lists_and_protect_cross_model_dependencies() {
    block_on(async {
        let repository = MemoryPromptResourceRepository::default();
        let service = PromptChunkService::new(repository);
        let mut base_request = request(None, "base", "detail");
        base_request.models = vec![
            ImageModel::NaiDiffusion45Full,
            ImageModel::NaiDiffusion5Full,
        ];
        let base = service.upsert_chunk(base_request).await.unwrap();
        let mut dependent_request = request(None, "scene", "$chunk(base)");
        dependent_request.models = vec![
            ImageModel::NaiDiffusion45Full,
            ImageModel::NaiDiffusion5Full,
        ];
        service.upsert_chunk(dependent_request).await.unwrap();

        assert_eq!(
            service
                .list_chunks(Some(ImageModel::NaiDiffusion5Full))
                .await
                .unwrap()
                .len(),
            2
        );

        let shrink = service
            .upsert_chunk(UpsertPromptChunkRequest {
                chunk_id: Some(base.id),
                models: vec![ImageModel::NaiDiffusion45Full],
                ..request(None, "base", "detail")
            })
            .await
            .unwrap_err();
        assert_eq!(shrink.kind(), PromptResourceErrorKind::Conflict);

        service
            .upsert_chunk(request(None, "v4-only", "legacy detail"))
            .await
            .unwrap();
        let incompatible = service
            .upsert_chunk(UpsertPromptChunkRequest {
                models: vec![ImageModel::NaiDiffusion5Full],
                ..request(None, "v5-scene", "$chunk(v4-only)")
            })
            .await
            .unwrap_err();
        assert_eq!(incompatible.kind(), PromptResourceErrorKind::Conflict);
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
        models: vec![ImageModel::NaiDiffusion45Full],
    }
}
