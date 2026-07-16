use super::*;

#[test]
fn commands_require_open_workspace_and_close_session() {
    block_on(async {
        let host = test_host();
        assert!(host.workspace_status().unwrap().is_none());

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
        assert_eq!(host.workspace_status().unwrap().unwrap().root, temp.path());

        let closed = host.close_workspace().unwrap();
        assert!(closed.was_open);
        assert!(host.workspace_status().unwrap().is_none());

        let bootstrap = host.bootstrap_app().await.unwrap();
        assert_eq!(bootstrap.workspace.unwrap().root, temp.path());
    });
}

#[test]
fn bootstrap_preserves_failed_recent_workspace_for_retry() {
    block_on(async {
        let invalid_root = tempfile::NamedTempFile::new().unwrap();
        let root = invalid_root.path().to_path_buf();
        let host = test_host_with_global_settings(GlobalSettings {
            last_workspace: Some(root.clone()),
            ..GlobalSettings::default()
        });

        let bootstrap = host.bootstrap_app().await.unwrap();

        assert!(bootstrap.workspace.is_none());
        let failure = bootstrap.restore_failure.unwrap();
        assert_eq!(failure.root, root);
        assert_eq!(bootstrap.global_settings.last_workspace, Some(failure.root));
        assert!(host.workspace_status().unwrap().is_none());
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
fn prompt_preset_commands_share_session() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let preset = host
            .upsert_prompt_preset(UpsertPromptPresetRequestDto {
                preset_id: None,
                kind: PromptPresetKindDto::Main,
                name: "Main".to_owned(),
                category: None,
                description: None,
                order: 0,
                enabled: true,
                before: "$chunk(hero)".to_owned(),
                after: "sharp focus".to_owned(),
                replace: String::new(),
                uc_before: String::new(),
                uc_after: String::new(),
                uc_replace: String::new(),
                quality_override: Some("qualityTagsV4".to_owned()),
                uc_preset_override: Some("heavy".to_owned()),
                preview: None,
            })
            .await
            .unwrap();

        assert_eq!(preset.kind, PromptPresetKindDto::Main);
        assert_eq!(
            host.list_prompt_presets(ListPromptPresetsRequestDto {
                kind: Some(PromptPresetKindDto::Main),
                include_disabled: false,
                offset: 0,
                limit: 10,
            })
            .await
            .unwrap()
            .total,
            1
        );
    });
}

#[test]
fn generation_prompt_compile_respects_requested_depth() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        upsert_hero_chunk(&host).await;

        let error = host
            .compile_generation_prompt_preview(CompileGenerationPromptRequestDto {
                prompt: "$chunk(hero)".to_owned(),
                main_preset_id: None,
                negative_prompt: None,
                characters: Vec::new(),
                max_depth: 0,
            })
            .await
            .unwrap_err();

        assert_eq!(error.code, "prompt_conflict");
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

        let release_request = ReleaseImportedImageResourcesRequestDto {
            resources: vec![imported.resource.clone()],
        };
        let released = host
            .release_imported_image_resources(ReleaseImportedImageResourcesRequestDto {
                resources: vec![imported.resource.clone()],
            })
            .await
            .unwrap();
        assert_eq!(released.released, 1);
        assert_eq!(released.resources_deleted, 1);
        assert_eq!(released.blobs_deleted, 1);
        assert!(
            host.get_resource_image(GetResourceImageRequestDto {
                resource: imported.resource,
            })
            .await
            .is_err()
        );
        assert_eq!(
            host.release_imported_image_resources(release_request)
                .await
                .unwrap()
                .released,
            0
        );
    });
}

#[test]
fn reopening_closed_workspace_cleans_stale_imported_images() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        let imported = host
            .import_image_resource(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::ReferenceImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap();

        host.close_workspace().unwrap();
        open_workspace(&host, &temp).await;

        assert!(
            host.get_resource_image(GetResourceImageRequestDto {
                resource: imported.resource,
            })
            .await
            .is_err()
        );
    });
}

#[test]
fn generation_draft_promotes_resources_survives_reopen_and_clears_owners() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;
        let imported = host
            .import_image_resource(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::ControlNetImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap()
            .resource;
        let defaults = host.get_workspace_settings().await.unwrap().generation;
        let draft = GenerationDraftDto {
            main_preset_id: None,
            prompt: "1girl".to_owned(),
            negative_prompt: String::new(),
            model: defaults.model,
            size: defaults.size,
            quality: defaults.quality,
            uc_preset: defaults.uc_preset,
            steps: defaults.steps,
            scale: defaults.scale,
            sampler: defaults.sampler,
            noise_schedule: defaults.noise_schedule,
            seed_mode: GenerationDraftSeedModeDto::Random,
            seed: defaults.seed,
            n_samples: defaults.n_samples,
            request_count: 1,
            cfg_rescale: defaults.cfg_rescale,
            variety_boost: defaults.variety_boost,
            image_format: defaults.image_format,
            strict_mode: defaults.strict_mode,
            stream_enabled: true,
            i2i: None,
            vibe: GenerationDraftVibeDto {
                enabled: true,
                strength: 1.0,
                slots: vec![GenerationDraftVibeSlotDto {
                    id: "vibe-slot".to_owned(),
                    encoding: imported.clone(),
                    vibe_id: None,
                    information_extracted: 1.0,
                    strength: 1.0,
                    display_name: "Imported source".to_owned(),
                    source_image: Some(imported.clone()),
                    source_sha256: None,
                }],
            },
            precise_references: Vec::new(),
            characters: Vec::new(),
            character_position_mode: GenerationDraftCharacterPositionModeDto::Global,
        };

        assert_eq!(
            host.save_generation_draft(SaveGenerationDraftRequestDto {
                draft: draft.clone(),
            })
            .await
            .unwrap(),
            draft
        );
        assert_eq!(
            host.release_imported_image_resources(ReleaseImportedImageResourcesRequestDto {
                resources: vec![imported.clone()],
            })
            .await
            .unwrap()
            .released,
            0
        );

        host.close_workspace().unwrap();
        open_workspace(&host, &temp).await;
        assert_eq!(host.get_generation_draft().await.unwrap(), Some(draft));
        host.get_resource_image(GetResourceImageRequestDto {
            resource: imported.clone(),
        })
        .await
        .unwrap();

        host.clear_generation_draft().await.unwrap();
        assert_eq!(host.get_generation_draft().await.unwrap(), None);
        assert!(
            host.get_resource_image(GetResourceImageRequestDto { resource: imported })
                .await
                .is_err()
        );
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

#[test]
fn global_settings_preserve_last_workspace_and_update_frontend_independently() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        open_workspace(&host, &temp).await;

        let settings = host.get_global_settings().await.unwrap();
        assert_eq!(settings.last_workspace.as_deref(), Some(temp.path()));
        assert!(!settings.frontend.developer_mode);
        assert!(!settings.frontend.gallery.blur_sensitive_images);

        let updated = host
            .update_global_settings(UpdateGlobalSettingsRequestDto {
                frontend: GlobalFrontendSettingsDto {
                    language: atelier_app_api::settings::FrontendLanguageDto::SimplifiedChinese,
                    developer_mode: true,
                    gallery: GlobalGallerySettingsDto {
                        blur_sensitive_images: true,
                    },
                },
            })
            .await
            .unwrap();
        assert_eq!(updated.last_workspace.as_deref(), Some(temp.path()));
        assert!(updated.frontend.developer_mode);
        assert!(updated.frontend.gallery.blur_sensitive_images);

        host.close_workspace().unwrap();
        let bootstrap = host.bootstrap_app().await.unwrap();
        assert_eq!(bootstrap.workspace.unwrap().root, temp.path());
        assert!(bootstrap.global_settings.frontend.developer_mode);
        assert!(
            bootstrap
                .global_settings
                .frontend
                .gallery
                .blur_sensitive_images
        );
    });
}
