use super::*;

#[test]
fn waiting_worker_cannot_advance_replacement_workspace() {
    block_on(async {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let host = test_host_with_factory(RecordingFactory::rate_limited_once());
        open_workspace(&host, &first).await;
        create_active_key(&host).await;
        upsert_hero_chunk(&host).await;
        host.submit_generation(submit_request("batch", "job"))
            .await
            .unwrap();
        host.run_generation_job(RunGenerationJobRequestDto {
            job_id: "job".to_owned(),
        })
        .await
        .unwrap();
        let worker = host.drive_generation_queue(
            QueueDirectiveDto::Wait {
                delay: atelier_app_api::generation::QueueDelayDto {
                    min_ms: 20,
                    max_ms: 20,
                },
            },
            GenerationWorkerCancel::new(),
        );
        futures::pin_mut!(worker);
        assert!(futures::poll!(&mut worker).is_pending());
        open_workspace(&host, &second).await;
        upsert_hero_chunk(&host).await;
        host.submit_generation(submit_request("batch", "job"))
            .await
            .unwrap();
        assert_eq!(worker.await.unwrap(), QueueDirectiveDto::Idle);
        let status = host
            .generation_status(GenerationStatusQueryDto {
                job_id: Some("job".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(status.job_status.as_deref(), Some("queued"));
    });
}
