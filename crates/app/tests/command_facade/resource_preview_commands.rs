use super::*;

#[test]
fn prompt_previews_survive_reopen_and_are_released_with_their_resources() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        let (chunk, preset, chunk_preview, preset_preview) =
            create_prompt_resources_with_previews(&host).await;

        assert_eq!(
            host.release_imported_image_resources(ReleaseImportedImageResourcesRequestDto {
                resources: vec![chunk_preview.clone(), preset_preview.clone()],
            })
            .await
            .unwrap()
            .released,
            0
        );

        host.close_workspace().unwrap();
        open_workspace(&host, &temp).await;

        let reopened_chunk = host
            .get_prompt_chunk(GetPromptChunkRequestDto {
                chunk_id: Some(chunk.chunk_id.clone()),
                key: None,
            })
            .await
            .unwrap();
        let reopened_preset = host
            .list_prompt_presets(ListPromptPresetsRequestDto {
                kind: Some(PromptPresetKindDto::Main),
                model: None,
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.preset_id == preset.preset_id)
            .unwrap();
        assert_eq!(reopened_chunk.preview.as_ref(), Some(&chunk_preview));
        assert_eq!(reopened_preset.preview.as_ref(), Some(&preset_preview));
        host.get_resource_image(GetResourceImageRequestDto {
            resource: chunk_preview.clone(),
        })
        .await
        .unwrap();
        host.get_resource_image(GetResourceImageRequestDto {
            resource: preset_preview.clone(),
        })
        .await
        .unwrap();

        host.upsert_prompt_chunk(UpsertPromptChunkRequestDto {
            chunk_id: Some(reopened_chunk.chunk_id),
            key: reopened_chunk.key,
            content: reopened_chunk.content,
            category: reopened_chunk.category,
            description: reopened_chunk.description,
            models: reopened_chunk.models,
            preview: None,
        })
        .await
        .unwrap();
        host.delete_prompt_preset(DeletePromptPresetRequestDto {
            preset_id: reopened_preset.preset_id,
        })
        .await
        .unwrap();

        assert!(
            host.get_resource_image(GetResourceImageRequestDto {
                resource: chunk_preview,
            })
            .await
            .is_err()
        );
        assert!(
            host.get_resource_image(GetResourceImageRequestDto {
                resource: preset_preview,
            })
            .await
            .is_err()
        );
    });
}

async fn create_prompt_resources_with_previews(
    host: &AtelierRuntime<MemorySecretStore, RecordingFactory>,
) -> (
    PromptChunkDto,
    PromptPresetDto,
    ResourceRefDto,
    ResourceRefDto,
) {
    let chunk_preview = host
        .import_image_resource(ImportImageResourceRequestDto {
            kind: ImageResourceKindDto::SourceImage,
            image_base64: "AQID".to_owned(),
            mime_type: Some("image/png".to_owned()),
        })
        .await
        .unwrap()
        .resource;
    let preset_preview = host
        .import_image_resource(ImportImageResourceRequestDto {
            kind: ImageResourceKindDto::SourceImage,
            image_base64: "BAUG".to_owned(),
            mime_type: Some("image/png".to_owned()),
        })
        .await
        .unwrap()
        .resource;
    let chunk = host
        .upsert_prompt_chunk(UpsertPromptChunkRequestDto {
            chunk_id: None,
            key: "previewed".to_owned(),
            content: "1girl".to_owned(),
            category: None,
            description: None,
            models: vec![ImageModelDto::NaiDiffusion45Full],
            preview: Some(chunk_preview.clone()),
        })
        .await
        .unwrap();
    let preset = host
        .upsert_prompt_preset(UpsertPromptPresetRequestDto {
            preset_id: None,
            kind: PromptPresetKindDto::Main,
            name: "Previewed".to_owned(),
            category: None,
            description: None,
            order: 0,
            prompt_behavior: PromptPresetBehaviorDto::Surround {
                before: String::new(),
                after: String::new(),
            },
            uc_behavior: PromptPresetBehaviorDto::Surround {
                before: String::new(),
                after: String::new(),
            },
            quality_override: None,
            uc_preset_override: None,
            models: vec![ImageModelDto::NaiDiffusion45Full],
            preview: Some(preset_preview.clone()),
        })
        .await
        .unwrap();
    (chunk, preset, chunk_preview, preset_preview)
}

#[test]
fn vibe_preview_survives_workspace_reopen() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let imported = host
            .import_vibe_document(ImportVibeDocumentRequestDto {
                file_name: "image-style.naiv4vibe".to_owned(),
                content: official_image_vibe("Image Style"),
            })
            .await
            .unwrap()
            .entries
            .into_iter()
            .next()
            .unwrap();
        let preview = imported.preview.unwrap();

        host.close_workspace().unwrap();
        open_workspace(&host, &temp).await;

        let reopened = host
            .list_vibe_documents(ListVibeDocumentsRequestDto::default())
            .await
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.vibe_id == imported.vibe_id)
            .unwrap();
        assert_eq!(reopened.preview.as_ref(), Some(&preview));
        host.get_resource_image(GetResourceImageRequestDto { resource: preview })
            .await
            .unwrap();
    });
}
