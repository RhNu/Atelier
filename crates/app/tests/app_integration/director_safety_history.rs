use super::*;

#[test]
fn director_tool_uses_resource_inputs_and_indexes_gallery_result() {
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

        let result = app
            .director()
            .run_tool(RunDirectorToolRequestDto {
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
        assert_eq!(result.item.source_kind, GallerySourceKindDto::Director);
        assert_eq!(result.resource.id, "resource:director:run-1");
        let director_request = &factory.director_requests()[0];
        assert_eq!(director_request.image, "AQID");
        assert_eq!(director_request.prompt, None);
        assert_eq!(director_request.defry, None);
        assert_eq!(
            app.gallery()
                .query(GalleryQueryDto::default())
                .await
                .unwrap()
                .items[0]
                .item_id,
            result.item_id
        );
    });
}

#[test]
fn injected_safety_scanner_scores_generated_gallery_items() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let image_bytes = valid_png_bytes(2, 1);
        let scanner = Arc::new(RecordingSafetyScanner::default());
        let app = WorkspaceSession::open_workspace_with_dependencies_and_safety_scanner(
            temp.path().to_path_buf(),
            MemorySecretStore::default(),
            RecordingFactory::with_image_bytes(image_bytes.clone()),
            Some(scanner.clone()),
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

        app.generation()
            .submit(submit_request("batch-1", "job-1", "1girl"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let gallery = app
            .gallery()
            .query(GalleryQueryDto::default())
            .await
            .unwrap();
        let safety = gallery.items[0].safety.as_ref().unwrap();
        assert_eq!(safety.nsfw_score, Some(0.91));
        assert_eq!(safety.safe_score, Some(0.09));
        assert_eq!(safety.risk_band, Some(GallerySafetyRiskBandDto::High));
        assert_eq!(safety.auto_label, Some(GallerySafetyLabelDto::Sensitive));
        assert_eq!(safety.effective_label, GallerySafetyLabelDto::Sensitive);
        assert_eq!(safety.raw_scores.len(), 2);
        assert_eq!(scanner.inputs(), vec![image_bytes]);
    });
}

#[test]
fn run_history_exposes_generation_outputs_and_reruns_as_new_jobs() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(2, 1)).await;

        app.generation()
            .submit(submit_request("batch-1", "job-1", "1girl"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let history = app
            .history()
            .query(RunHistoryQueryDto {
                kind: Some(RunHistoryKindDto::Generation),
                offset: 0,
                limit: 10,
                ..RunHistoryQueryDto::default()
            })
            .await
            .unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].run_id, "job-1");
        assert_eq!(history.items[0].outputs[0].asset_role, "original");

        let duplicate = app
            .history()
            .rerun_generation(RerunGenerationHistoryItemRequestDto {
                run_id: "job-1".to_owned(),
                batch_id: "batch-2".to_owned(),
                job_id: "job-1".to_owned(),
            })
            .await
            .unwrap_err();
        assert_eq!(duplicate.code(), "invalid_request");

        let image = app
            .resources()
            .get_image(GetResourceImageRequestDto {
                resource: history.items[0].outputs[0].resource.clone(),
            })
            .await
            .unwrap();
        assert_eq!(image.mime_type.as_deref(), Some("image/png"));
        assert!(!image.image_base64.is_empty());

        let rerun = app
            .history()
            .rerun_generation(RerunGenerationHistoryItemRequestDto {
                run_id: "job-1".to_owned(),
                batch_id: "batch-2".to_owned(),
                job_id: "job-2".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(rerun.item.origin_run_id.as_deref(), Some("job-1"));
        app.generation().run_job("job-2").await.unwrap();

        let history = app
            .history()
            .query(RunHistoryQueryDto::default())
            .await
            .unwrap();
        assert_eq!(history.total, 2);
        assert!(history.items.iter().any(|item| item.run_id == "job-2"));
    });
}

#[test]
fn generation_submit_rejects_durable_history_ids() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_image(&temp, valid_png_bytes(2, 1)).await;

        app.generation()
            .submit(submit_request("batch-1", "job-1", "1girl"))
            .await
            .unwrap();
        app.generation().run_job("job-1").await.unwrap();

        let duplicate_job = app
            .generation()
            .submit(submit_request("batch-2", "job-1", "1girl"))
            .await
            .unwrap_err();
        assert_eq!(duplicate_job.code(), "invalid_request");

        let duplicate_batch = app
            .generation()
            .submit(submit_request("batch-1", "job-2", "1girl"))
            .await
            .unwrap_err();
        assert_eq!(duplicate_batch.code(), "invalid_request");

        let history = app
            .history()
            .query(RunHistoryQueryDto::default())
            .await
            .unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].run_id, "job-1");
        assert_eq!(history.items[0].status, RunHistoryStatusDto::Succeeded);
    });
}

#[test]
fn run_history_records_director_outputs_without_queueing() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            MemorySecretStore::default(),
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

        app.director()
            .run_tool(RunDirectorToolRequestDto {
                run_id: "run-1".to_owned(),
                tool: DirectorToolDto::Lineart,
                image: ImageInputDto::ResourceRef { resource: source },
                prompt: None,
                defry: None,
                strict_mode: true,
            })
            .await
            .unwrap();

        let history = app
            .history()
            .query(RunHistoryQueryDto {
                kind: Some(RunHistoryKindDto::Director),
                offset: 0,
                limit: 10,
                ..RunHistoryQueryDto::default()
            })
            .await
            .unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].run_id, "run-1");
        assert_eq!(history.items[0].outputs[0].asset_role, "original");
        assert_eq!(
            app.generation().status(None).await.batch_status.as_deref(),
            None
        );
    });
}

#[test]
fn run_history_records_failed_director_runs() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory =
            RecordingFactory::with_director_error(DirectorClientError::transport("director down"));
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            MemorySecretStore::default(),
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

        let error = app
            .director()
            .run_tool(RunDirectorToolRequestDto {
                run_id: "run-failed".to_owned(),
                tool: DirectorToolDto::Lineart,
                image: ImageInputDto::ResourceRef { resource: source },
                prompt: None,
                defry: None,
                strict_mode: true,
            })
            .await
            .unwrap_err();
        assert_eq!(error.code(), "kernel");

        let history = app
            .history()
            .query(RunHistoryQueryDto {
                kind: Some(RunHistoryKindDto::Director),
                offset: 0,
                limit: 10,
                ..RunHistoryQueryDto::default()
            })
            .await
            .unwrap();
        assert_eq!(history.total, 1);
        assert_eq!(history.items[0].run_id, "run-failed");
        assert_eq!(history.items[0].status, RunHistoryStatusDto::Failed);
        assert!(history.items[0].last_error.is_some());
        assert!(history.items[0].outputs.is_empty());
    });
}
