use super::*;

#[test]
fn open_workspace_initializes_lexicon_and_generation_is_explicitly_driven() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let secrets = MemorySecretStore::default();
        let factory = RecordingFactory::default();
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets,
            factory.clone(),
        )
        .await
        .unwrap();

        assert!(
            !app.prompt()
                .lexicon_search("1girl", 5)
                .unwrap()
                .items
                .is_empty()
        );

        let missing_key = app
            .generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap_err();
        assert_eq!(missing_key.code(), "missing_active_key");

        app.account()
            .create_api_key(CreateApiKeyRequestDto {
                id: "main".to_owned(),
                display_name: "Main".to_owned(),
                secret: "active-secret".to_owned(),
            })
            .await
            .unwrap();
        app.account().set_active_api_key("main").await.unwrap();
        app.prompt()
            .upsert_chunk(atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        let directive = app
            .generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        assert_eq!(
            directive,
            QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned()
            }
        );
        assert!(factory.secrets().is_empty());

        app.generation().run_job("job-1").await.unwrap();

        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        assert_eq!(
            app.gallery()
                .query(GalleryQueryDto::default())
                .await
                .unwrap()
                .items[0]
                .assets
                .iter()
                .map(|asset| asset.role.as_str())
                .collect::<Vec<_>>(),
            vec!["original"]
        );
    });
}

#[test]
fn valid_generated_images_get_best_effort_gallery_variants() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let secrets = MemorySecretStore::default();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets,
            factory,
        )
        .await
        .unwrap();

        app.account()
            .create_api_key(CreateApiKeyRequestDto {
                id: "main".to_owned(),
                display_name: "Main".to_owned(),
                secret: "active-secret".to_owned(),
            })
            .await
            .unwrap();
        app.account().set_active_api_key("main").await.unwrap();
        app.prompt()
            .upsert_chunk(atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        assert_eq!(gallery.items.len(), 1);
        let item = &gallery.items[0];
        assert_eq!(
            item.assets
                .iter()
                .map(|asset| (asset.role.as_str(), asset.variant_kind.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("original", Some("original")),
                ("thumbnail", Some("thumbnail")),
                ("preview", Some("preview")),
                ("sanitized", Some("sanitized")),
                ("export", Some("export")),
            ]
        );
        assert!(!app.events().events_since(0, 100).is_empty());
    });
}

#[test]
#[allow(clippy::too_many_lines)]
fn generation_submit_applies_prompt_presets_before_queueing_work() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let secrets = MemorySecretStore::default();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets,
            factory.clone(),
        )
        .await
        .unwrap();

        app.account()
            .create_api_key(CreateApiKeyRequestDto {
                id: "main".to_owned(),
                display_name: "Main".to_owned(),
                secret: "active-secret".to_owned(),
            })
            .await
            .unwrap();
        app.account().set_active_api_key("main").await.unwrap();
        app.prompt()
            .upsert_chunk(atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "lighting".to_owned(),
                content: "cinematic lighting".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();
        let main = app
            .prompt()
            .upsert_preset(atelier_app_api::prompt::UpsertPromptPresetRequestDto {
                preset_id: None,
                kind: atelier_app_api::prompt::PromptPresetKindDto::Main,
                name: "Main".to_owned(),
                category: None,
                description: None,
                order: 0,
                enabled: true,
                before: "@chunk(lighting)".to_owned(),
                after: "sharp focus".to_owned(),
                replace: String::new(),
                uc_before: "bad anatomy".to_owned(),
                uc_after: String::new(),
                uc_replace: String::new(),
                quality_override: None,
                uc_preset_override: Some("heavy".to_owned()),
                preview: None,
            })
            .await
            .unwrap();
        let character = app
            .prompt()
            .upsert_preset(atelier_app_api::prompt::UpsertPromptPresetRequestDto {
                preset_id: None,
                kind: atelier_app_api::prompt::PromptPresetKindDto::Character,
                name: "Hero".to_owned(),
                category: None,
                description: None,
                order: 0,
                enabled: true,
                before: "red hair".to_owned(),
                after: String::new(),
                replace: String::new(),
                uc_before: String::new(),
                uc_after: "extra arms".to_owned(),
                uc_replace: String::new(),
                quality_override: None,
                uc_preset_override: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(SubmitGenerationRequestDto {
                batch_id: "batch-1".to_owned(),
                job_id: "job-1".to_owned(),
                work: GenerationWorkRequestDto::Image(GenerateImageRequestDto {
                    main_preset_id: Some(main.preset_id),
                    prompt: "1girl".to_owned(),
                    negative_prompt: Some("lowres".to_owned()),
                    characters: Some(vec![atelier_app_api::generation::CharacterDto {
                        preset_id: Some(character.preset_id),
                        prompt: "solo".to_owned(),
                        negative_prompt: Some("worst quality".to_owned()),
                        position: atelier_app_api::generation::CharacterPositionDto::default(),
                        enabled: true,
                    }]),
                    ..GenerateImageRequestDto::default()
                }),
                context: GenerationPlanContextDto::default(),
            })
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let request = factory.generated_requests().remove(0);
        assert_eq!(request.prompt, "cinematic lighting, 1girl, sharp focus");
        assert_eq!(
            request.negative_prompt,
            Some("bad anatomy, lowres".to_owned())
        );
        assert_eq!(request.uc_preset, atelier_generation::UcPreset::Heavy);
        let character = request.characters.unwrap().remove(0);
        assert_eq!(character.prompt, "red hair, solo");
        assert_eq!(
            character.negative_prompt,
            Some("worst quality, extra arms".to_owned())
        );
    });
}

#[test]
fn generation_estimate_applies_character_prompt_presets() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            MemorySecretStore::default(),
            RecordingFactory::default(),
        )
        .await
        .unwrap();
        let character = app
            .prompt()
            .upsert_preset(atelier_app_api::prompt::UpsertPromptPresetRequestDto {
                preset_id: None,
                kind: atelier_app_api::prompt::PromptPresetKindDto::Character,
                name: "Hero".to_owned(),
                category: None,
                description: None,
                order: 0,
                enabled: true,
                before: "red hair".to_owned(),
                after: String::new(),
                replace: String::new(),
                uc_before: String::new(),
                uc_after: String::new(),
                uc_replace: String::new(),
                quality_override: None,
                uc_preset_override: None,
                preview: None,
            })
            .await
            .unwrap();

        let estimate = app
            .generation()
            .estimate(GenerationEstimateRequestDto {
                request: GenerateImageRequestDto {
                    prompt: "1girl".to_owned(),
                    characters: Some(vec![atelier_app_api::generation::CharacterDto {
                        preset_id: Some(character.preset_id),
                        prompt: String::new(),
                        negative_prompt: None,
                        position: atelier_app_api::generation::CharacterPositionDto::default(),
                        enabled: true,
                    }]),
                    ..GenerateImageRequestDto::default()
                },
                context: GenerationPlanContextDto::default(),
            })
            .await
            .unwrap();

        assert!(estimate.total_cost > 0);
    });
}

#[test]
fn valid_streamed_images_get_best_effort_gallery_variants() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(2, 1)).await;
        app.prompt()
            .upsert_chunk(atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(stream_submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        assert_eq!(gallery.items.len(), 1);
        assert_eq!(
            asset_roles_and_kinds(&gallery.items[0]),
            vec![
                ("original", Some("original")),
                ("thumbnail", Some("thumbnail")),
                ("preview", Some("preview")),
                ("sanitized", Some("sanitized")),
                ("export", Some("export")),
            ]
        );
    });
}

#[test]
fn updated_variant_settings_drive_generated_variant_dimensions() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(1200, 600)).await;
        let mut settings = app.settings().get().await.unwrap();
        settings.image_variants = ImageVariantSettingsDto {
            thumbnail_long_edge: 160,
            preview_long_edge: 640,
        };
        app.settings()
            .update(UpdateWorkspaceSettingsRequestDto { settings })
            .await
            .unwrap();
        app.prompt()
            .upsert_chunk(atelier_app_api::prompt::UpsertPromptChunkRequestDto {
                chunk_id: None,
                key: "hero".to_owned(),
                content: "1girl".to_owned(),
                category: None,
                description: None,
                preview: None,
            })
            .await
            .unwrap();

        app.generation()
            .submit(submit_request("batch-1", "job-1", "@chunk(hero)"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        let item = &gallery.items[0];
        let repository = DatabaseResourceCatalogRepository::new(
            DatabaseConnection::open(workspace_database_path(&WorkspaceRoot::new(
                temp.path().to_path_buf(),
            )))
            .unwrap(),
        );
        let thumbnail = variant_by_role(item, "thumbnail");
        let preview = variant_by_role(item, "preview");

        let thumbnail = repository
            .get_variant(&VariantId::new(thumbnail))
            .await
            .unwrap()
            .unwrap();
        let preview = repository
            .get_variant(&VariantId::new(preview))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(thumbnail.metadata.mime_type.as_deref(), Some("image/webp"));
        assert_eq!(
            (thumbnail.metadata.width, thumbnail.metadata.height),
            (Some(160), Some(80))
        );
        assert_eq!(preview.metadata.mime_type.as_deref(), Some("image/webp"));
        assert_eq!(
            (preview.metadata.width, preview.metadata.height),
            (Some(640), Some(320))
        );
    });
}

#[test]
fn resource_backed_generation_inputs_are_resolved_before_novelai_submission() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            MemorySecretStore::default(),
            factory.clone(),
        )
        .await
        .unwrap();
        app.account()
            .create_api_key(CreateApiKeyRequestDto {
                id: "main".to_owned(),
                display_name: "Main".to_owned(),
                secret: "active-secret".to_owned(),
            })
            .await
            .unwrap();
        app.account().set_active_api_key("main").await.unwrap();

        let source = app
            .resources()
            .import_image(ImportImageResourceRequestDto {
                kind: ImageResourceKindDto::SourceImage,
                image_base64: "AQID".to_owned(),
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap()
            .resource;

        app.generation()
            .submit(SubmitGenerationRequestDto {
                batch_id: "batch-1".to_owned(),
                job_id: "job-1".to_owned(),
                work: GenerationWorkRequestDto::Image(GenerateImageRequestDto {
                    prompt: "1girl".to_owned(),
                    i2i: Some(Img2ImgRequestDto {
                        image: ImageInputDto::ResourceRef {
                            resource: source.clone(),
                        },
                        strength: 0.5,
                        noise: 0.2,
                        mask: None,
                    }),
                    ..GenerateImageRequestDto::default()
                }),
                context: GenerationPlanContextDto::default(),
            })
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let generated = factory.generated_requests();
        assert_eq!(generated.len(), 1);
        assert_eq!(generated[0].i2i.as_ref().unwrap().image, "AQID");
    });
}
