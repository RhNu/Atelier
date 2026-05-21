use super::*;

#[test]
fn generation_events_and_gallery_commands_share_session() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::default();
        let host = test_host_with_factory(factory.clone());
        open_workspace(&host, &temp).await;

        let missing_key = host
            .submit_generation(submit_request("batch-1", "job-1"))
            .await;
        assert_eq!(missing_key.unwrap_err().code, "missing_active_key");

        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;
        submit_and_run_generation(&host, "batch-1", "job-1").await;
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);

        factory.clear();
        submit_and_run_generation(&host, "batch-2", "job-2").await;
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);

        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job-2".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.batch_status.as_deref(), Some("succeeded"));
        assert_eq!(status.job_status.as_deref(), Some("succeeded"));

        let events = host
            .events_since(EventsSinceRequestDto {
                sequence: 0,
                limit: 100,
            })
            .unwrap();
        assert!(!events.items.is_empty());
        assert_eq!(
            events.next_sequence,
            events.items.last().map_or(0, |event| event.sequence)
        );

        let gallery = host
            .query_gallery(GalleryQueryDto {
                offset: 0,
                limit: 1,
                ..GalleryQueryDto::default()
            })
            .await
            .unwrap();
        assert_eq!(gallery.items.len(), 1);
        assert_eq!(gallery.total, 2);
        assert_eq!(gallery.items[0].artifact_kind, "generated_image");
        assert_eq!(
            host.query_gallery(GalleryQueryDto {
                offset: 0,
                limit: 10,
                artifact_kind: Some("generated_image".to_owned()),
                ..GalleryQueryDto::default()
            })
            .await
            .unwrap()
            .total,
            2
        );
        let item_id = gallery.items[0].item_id.clone();
        let reference = host
            .gallery_image_reference(GalleryImageReferenceRequestDto {
                item_id: item_id.clone(),
                target: GalleryImageReferenceTargetDto::PreciseReference,
            })
            .await
            .unwrap();
        assert_eq!(reference.item_id, item_id);
        assert_eq!(
            reference.target,
            GalleryImageReferenceTargetDto::PreciseReference
        );
        assert_eq!(reference.asset_role, "original");

        let overridden = host
            .set_gallery_safety_override(SetGallerySafetyOverrideRequestDto {
                item_id,
                manual_safety_override: Some(GallerySafetyOverrideDto::Hidden),
            })
            .await
            .unwrap();
        assert_eq!(
            overridden.manual_safety_override,
            Some(GallerySafetyOverrideDto::Hidden)
        );
    });
}

#[test]
fn generation_worker_drives_submitted_job_to_idle() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let factory = RecordingFactory::default();
        let host = test_host_with_factory(factory.clone());
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;

        let directive = host
            .submit_generation(submit_request("batch-worker", "job-worker"))
            .await
            .unwrap();
        let final_directive = host
            .drive_generation_queue(directive, GenerationWorkerCancel::new())
            .await
            .unwrap();

        assert_eq!(final_directive, QueueDirectiveDto::Idle);
        assert_eq!(factory.secrets(), vec!["active-secret".to_owned()]);
        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job-worker".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.batch_status.as_deref(), Some("succeeded"));
        assert_eq!(status.job_status.as_deref(), Some("succeeded"));
        assert_eq!(
            host.query_gallery(GalleryQueryDto::default())
                .await
                .unwrap()
                .total,
            1
        );
    });
}

#[test]
fn generation_worker_advances_zero_delay() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host_with_factory(RecordingFactory::rate_limited_once());
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;

        let directive = host
            .submit_generation(submit_request("batch-delay", "job-delay"))
            .await
            .unwrap();
        let final_directive = host
            .drive_generation_queue(directive, GenerationWorkerCancel::new())
            .await
            .unwrap();

        assert_eq!(final_directive, QueueDirectiveDto::Idle);
    });
}

#[test]
fn generation_worker_cancel_stops_during_wait_without_advancing_queue() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host_with_factory(RecordingFactory::rate_limited_once());
        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;

        host.submit_generation(submit_request("batch-cancel", "job-cancel"))
            .await
            .unwrap();
        let wait = host
            .run_generation_job(RunGenerationJobRequestDto {
                job_id: "job-cancel".to_owned(),
            })
            .await
            .unwrap();
        assert!(matches!(wait, QueueDirectiveDto::Wait { .. }));
        let cancel = GenerationWorkerCancel::new();
        cancel.cancel();

        let returned = host
            .drive_generation_queue(wait.clone(), cancel)
            .await
            .unwrap();

        assert_eq!(returned, wait);
        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job-cancel".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.batch_status.as_deref(), Some("waiting"));
    });
}

#[test]
fn event_subscription_receives_same_events_as_events_since() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        let received = Arc::new(Mutex::new(Vec::<AppEventDto>::new()));
        let received_events = Arc::clone(&received);
        host.subscribe_events(Arc::new(move |event| {
            received_events.lock().unwrap().push(event);
        }))
        .unwrap();

        open_workspace(&host, &temp).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;
        let directive = host
            .submit_generation(submit_request("batch-events", "job-events"))
            .await
            .unwrap();
        host.drive_generation_queue(directive, GenerationWorkerCancel::new())
            .await
            .unwrap();

        let events = host
            .events_since(EventsSinceRequestDto {
                sequence: 0,
                limit: 100,
            })
            .unwrap()
            .items;
        let pushed = received.lock().unwrap().clone();

        assert_eq!(pushed, events);
        assert!(pushed.iter().any(|event| {
            matches!(
                &event.kind,
                AppEventKindDto::JobSucceeded { job_id, .. } if job_id == "job-events"
            )
        }));
    });
}

#[test]
fn prompt_lexicon_and_vibe_commands_are_available_through_facade() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let host = test_host();
        host.open_workspace(OpenWorkspaceRequestDto {
            root: temp.path().to_path_buf(),
        })
        .await
        .unwrap();
        host.create_api_key(CreateApiKeyRequestDto {
            id: "main".to_owned(),
            display_name: "Main".to_owned(),
            secret: "active-secret".to_owned(),
        })
        .await
        .unwrap();
        host.set_active_api_key(SetActiveApiKeyRequestDto {
            id: "main".to_owned(),
        })
        .await
        .unwrap();

        assert!(
            !host
                .prompt_lexicon_search(PromptLexiconSearchQueryDto {
                    query: "1girl".to_owned(),
                    limit: 5,
                })
                .unwrap()
                .items
                .is_empty()
        );

        let imported = host
            .import_vibe_document(ImportVibeDocumentRequestDto {
                file_name: "style.naiv4vibe".to_owned(),
                content: official_vibe("Style A"),
            })
            .await
            .unwrap();
        assert_eq!(imported.entries.len(), 1);
        let vibe_id = imported.entries[0].vibe_id.clone();

        let exported = host
            .export_vibe_document(ExportVibeDocumentRequestDto {
                vibe_ids: vec![vibe_id.clone()],
                format: VibeExportFormatDto::Naiv4vibe,
            })
            .await
            .unwrap();
        assert_eq!(exported.file_extension, "naiv4vibe");
        assert!(exported.content.contains("Style A"));

        let ensured = host
            .ensure_vibe_encoding(EnsureVibeEncodingRequestDto {
                vibe_id,
                source_sha256: "source-sha".to_owned(),
                image: "source-image-base64".to_owned(),
                model: VibeModelDto::NaiDiffusion45Full,
                information_extracted: 0.7,
            })
            .await
            .unwrap();
        assert!(!ensured.resource.id.is_empty());
    });
}
