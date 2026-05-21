use super::*;

#[test]
fn commands_require_open_workspace_and_close_session() {
    block_on(async {
        let host = test_host();
        let error = host.workspace_status().unwrap_err();
        assert_eq!(error.code, "workspace_not_open");

        let temp = tempfile::tempdir().unwrap();
        let status = host
            .open_workspace(OpenWorkspaceRequestDto {
                root: temp.path().to_path_buf(),
            })
            .await
            .unwrap();
        assert!(status.locked);
        let reopened = host
            .open_workspace(OpenWorkspaceRequestDto {
                root: temp.path().to_path_buf(),
            })
            .await
            .unwrap();
        assert!(reopened.locked);

        let invalid_root = tempfile::NamedTempFile::new().unwrap();
        let error = host
            .open_workspace(OpenWorkspaceRequestDto {
                root: invalid_root.path().to_path_buf(),
            })
            .await
            .unwrap_err();
        assert_ne!(error.code, "workspace_not_open");
        assert_eq!(host.workspace_status().unwrap().root, temp.path());

        let closed = host.close_workspace().unwrap();
        assert!(closed.was_open);
        let error = host.workspace_status().unwrap_err();
        assert_eq!(error.code, "workspace_not_open");
    });
}

#[test]
fn account_and_prompt_chunk_commands_share_session() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::default();
        let host = test_host_with_factory(factory.clone());
        open_workspace(&host, &temp).await;

        create_active_key(&host).await;
        assert!(factory.secrets().is_empty());
        let subscription = host
            .probe_api_key(ProbeApiKeyRequestDto {
                id: "main".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(subscription.anlas_balance, 100);
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        factory.clear();

        let hero = upsert_hero_chunk(&host).await;
        assert_eq!(
            host.get_prompt_chunk(GetPromptChunkRequestDto {
                chunk_id: None,
                key: Some("hero".to_owned()),
            })
            .await
            .unwrap()
            .content,
            "1girl"
        );
        assert_eq!(
            host.list_prompt_chunks(ListPromptChunksRequestDto {
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap()
            .total,
            1
        );
        let companion = upsert_scene_chunk(&host).await;
        let referenced_delete = host
            .delete_prompt_chunk(DeletePromptChunkRequestDto {
                chunk_id: hero.chunk_id.clone(),
            })
            .await;
        assert_eq!(referenced_delete.unwrap_err().code, "prompt_conflict");

        assert!(
            host.delete_prompt_chunk(DeletePromptChunkRequestDto {
                chunk_id: companion.chunk_id,
            })
            .await
            .unwrap()
            .deleted
        );
        assert!(
            host.delete_prompt_chunk(DeletePromptChunkRequestDto {
                chunk_id: hero.chunk_id,
            })
            .await
            .unwrap()
            .deleted
        );
    });
}

#[test]
fn resource_import_command_is_available_through_facade() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let imported = host
            .import_image_resource(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::SourceImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap();

        assert!(imported.resource.id.starts_with("resource:import:source:"));
        assert_eq!(imported.resource.variant_id, None);
    });
}

#[test]
fn director_command_is_available_through_facade() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        let source = host
            .import_image_resource(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::SourceImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap()
            .resource;

        let result = host
            .run_director_tool(RunDirectorToolRequestDto {
                run_id: "run-1".to_owned(),
                tool: DirectorToolDto::Lineart,
                image: ImageInputDto::ResourceRef { resource: source },
                prompt: Some("clean lines".to_owned()),
                defry: Some(2),
                strict_mode: true,
            })
            .await
            .unwrap();

        assert_eq!(result.artifact_id, "director:run-1");
        assert_eq!(result.item.artifact_kind, "director_result");
        assert_eq!(result.resource.id, "resource:director:run-1");
    });
}

#[test]
fn settings_commands_persist_across_workspace_reopen() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let defaults = host.get_workspace_settings().await.unwrap();
        assert_eq!(defaults.image_variants.thumbnail_long_edge, 320);
        assert_eq!(defaults.image_variants.preview_long_edge, 1024);

        let updated = WorkspaceSettingsDto {
            generation: GenerationDefaultsDto {
                model: ImageModelDto::NaiDiffusion4Curated,
                n_samples: 2,
                ..GenerationDefaultsDto::default()
            },
            image_variants: ImageVariantSettingsDto {
                thumbnail_long_edge: 256,
                preview_long_edge: 768,
            },
        };
        assert_eq!(
            host.update_workspace_settings(UpdateWorkspaceSettingsRequestDto {
                settings: updated.clone(),
            })
            .await
            .unwrap(),
            updated
        );

        host.close_workspace().unwrap();
        open_workspace(&host, &temp).await;
        assert_eq!(host.get_workspace_settings().await.unwrap(), updated);

        let reset = host.reset_workspace_settings().await.unwrap();
        assert_eq!(reset.settings, WorkspaceSettingsDto::default());
        assert_eq!(
            host.get_workspace_settings().await.unwrap(),
            WorkspaceSettingsDto::default()
        );
    });
}
