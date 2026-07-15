use super::*;

#[test]
fn generation_queue_recovers_as_paused_after_workspace_reopen() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let secrets = MemorySecretStore::default();
        let factory = RecordingFactory::with_image_bytes(valid_png_bytes(2, 1));
        let app = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets.clone(),
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
        app.generation()
            .submit(submit_request("batch-1", "job-1", "1girl"))
            .await
            .unwrap();
        drop(app);

        let reopened = WorkspaceSession::open_workspace_with_dependencies(
            temp.path().to_path_buf(),
            secrets,
            factory,
        )
        .await
        .unwrap();

        assert_eq!(
            reopened
                .generation()
                .status(Some("job-1"))
                .await
                .unwrap()
                .batch_status,
            Some("paused".to_owned())
        );
        let history = reopened
            .history()
            .query(RunHistoryQueryDto {
                status: Some(RunHistoryStatusDto::Paused),
                ..RunHistoryQueryDto::default()
            })
            .await
            .unwrap();
        assert_eq!(history.items.len(), 1);
        assert_eq!(history.items[0].run_id, "job-1");
        assert!(history.items[0].recoverable);
        assert_eq!(
            reopened.generation().resume().await.unwrap(),
            QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned()
            }
        );
    });
}
