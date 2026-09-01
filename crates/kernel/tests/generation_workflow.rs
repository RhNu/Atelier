mod support;

use std::time::Duration;

use atelier_generation::{
    CharacterPosition, GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage,
    GeneratedImageMetadata, GenerationClientError, GenerationPlanContext, ImageModel,
    ImageStreamEvent, ParsedGeneratedImageMetadata,
};
use atelier_jobs::{BatchId, JobId, JobStatus, QueueDelay, QueueDirective, RetryPolicy};
use atelier_kernel::{
    GenerationWorkRequest, KernelError, KernelEventKind, KernelFailureDetail, KernelRuntime,
    SubmitGenerationWork,
};
use atelier_resource_catalog::ResourceKind;
use base64::Engine;
use futures_executor::block_on;

use support::MemoryKernelPorts;

fn generated_image(bytes: Vec<u8>, seed: Option<i64>) -> GeneratedImage {
    let metadata = seed.map_or(GeneratedImageMetadata::NotPresent, |seed| {
        GeneratedImageMetadata::Parsed(ParsedGeneratedImageMetadata {
            prompt: Some("1girl".to_owned()),
            negative_prompt: None,
            seed: Some(seed),
            metadata_json: format!(r#"{{"seed":{seed}}}"#),
            warnings: Vec::new(),
        })
    });
    GeneratedImage {
        bytes,
        mime_type: Some("image/png".to_owned()),
        metadata,
    }
}

#[test]
fn submit_generation_work_only_enqueues_payload() {
    block_on(async {
        let ports = MemoryKernelPorts::default();
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-1");

        let directive = runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "$chunk(hero)"))
            .await
            .unwrap();

        assert_eq!(directive, QueueDirective::StartJob(job_id.clone()));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Queued));
        assert_eq!(ports.submitted_payload_count(), 1);
        assert_eq!(ports.compile_call_count(), 0);
        assert_eq!(ports.generate_call_count(), 0);
        assert!(matches!(
            ports.events()[0].kind,
            KernelEventKind::BatchSubmitted { .. }
        ));
    });
}

#[test]
fn rejected_submit_does_not_overwrite_existing_payload() {
    block_on(async {
        let ports = MemoryKernelPorts::default();
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "first prompt"))
            .await
            .unwrap();
        let error = runtime
            .submit_generation_work(image_work("batch-2", job_id.clone(), "second prompt"))
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::JobQueue(_)));
        assert_eq!(ports.submitted_payload_count(), 1);
        assert_eq!(
            ports.submitted_prompt("generation-submitted:job-1"),
            Some("first prompt".to_owned())
        );
        assert_eq!(
            ports
                .operations()
                .into_iter()
                .filter(|operation| operation == "save_submitted")
                .count(),
            1
        );
    });
}

#[test]
fn compile_failure_marks_preparing_job_failed() {
    block_on(async {
        let ports = MemoryKernelPorts::default().failing_compile_prompt();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::PromptResource(_)));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn planning_failure_marks_preparing_job_failed() {
    block_on(async {
        let ports = MemoryKernelPorts::default();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "   "))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::Generation(_)));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn prepared_payload_failure_marks_preparing_job_failed() {
    block_on(async {
        let ports = MemoryKernelPorts::default().failing_prepared_payload();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::PayloadStore(_)));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn image_generation_compiles_plans_persists_and_indexes_samples() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_expanded_prompt("expanded prompt")
            .with_generated_images(vec![generated_image(vec![1, 2, 3], Some(77))]);
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "$chunk(hero)"))
            .await
            .unwrap();
        let directive = runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        assert_eq!(directive, QueueDirective::Idle);
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Succeeded));
        assert_eq!(
            ports.operations(),
            vec![
                "save_submitted",
                "compile_prompt",
                "save_prepared",
                "generate",
                "register_resource:GeneratedImage",
                "register_artifact",
                "score_image",
                "index_gallery"
            ]
        );
        assert_eq!(
            ports.registered_resources()["resource:job-1:sample:0"].kind,
            ResourceKind::GeneratedImage
        );
        let artifact = &ports.artifacts()["artifact:job-1:sample:0"];
        assert_eq!(artifact.metadata.request_seed, Some(4242));
        assert_eq!(artifact.metadata.seed, Some(77));
        assert_eq!(
            artifact.metadata.embedded_metadata_status,
            Some(atelier_artifacts::EmbeddedMetadataStatus::Parsed)
        );
        assert_eq!(artifact.metadata.embedded_prompt.as_deref(), Some("1girl"));
        assert_eq!(artifact.metadata.sample_index, Some(0));
        assert_eq!(
            artifact.replay.as_ref().unwrap().prompt_snapshot.as_deref(),
            Some("expanded prompt")
        );
        assert!(
            ports
                .gallery_items()
                .contains_key("artifact:artifact:job-1:sample:0")
        );
        assert!(
            ports
                .events()
                .iter()
                .any(|event| matches!(event.kind, KernelEventKind::GalleryIndexed { .. }))
        );
    });
}

#[test]
fn generation_workflow_compiles_negative_and_character_prompt_scopes() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_compiled_prompt("$chunk(main)", "expanded main")
            .with_compiled_prompt("$chunk(uc)", "expanded uc")
            .with_compiled_prompt("$chunk(hero)", "expanded hero")
            .with_compiled_prompt("$chunk(hero_uc)", "expanded hero uc")
            .with_generated_images(vec![generated_image(vec![1], None)]);
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-compile-scopes");

        runtime
            .submit_generation_work(SubmitGenerationWork {
                batch_id: BatchId::new("batch-compile-scopes"),
                job_id: job_id.clone(),
                request: GenerationWorkRequest::Image(GenerateImageRequest {
                    prompt: "$chunk(main)".to_owned(),
                    negative_prompt: Some("$chunk(uc)".to_owned()),
                    characters: Some(vec![atelier_generation::Character {
                        prompt: "$chunk(hero)".to_owned(),
                        negative_prompt: Some("$chunk(hero_uc)".to_owned()),
                        position: CharacterPosition::default(),
                        enabled: true,
                    }]),
                    model: ImageModel::NaiDiffusion5Full,
                    ..Default::default()
                }),
                context: GenerationPlanContext::default(),
            })
            .await
            .unwrap();

        runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        // Every scope compiles against the requested model, not the default one.
        assert_eq!(ports.compiled_models(), [ImageModel::NaiDiffusion5Full; 4]);
        let request = ports.generated_requests().pop().expect("request recorded");
        assert_eq!(request.prompt, "expanded main");
        assert_eq!(request.negative_prompt.as_deref(), Some("expanded uc"));
        let characters = request.characters.expect("characters preserved");
        assert_eq!(characters[0].prompt, "expanded hero");
        assert_eq!(
            characters[0].negative_prompt.as_deref(),
            Some("expanded hero uc")
        );
    });
}

#[test]
fn streaming_generation_emits_chunks_and_persists_only_latest_sample_frames() {
    block_on(async {
        let first = base64::engine::general_purpose::STANDARD.encode([1, 1]);
        let latest = base64::engine::general_purpose::STANDARD.encode([2, 2, 2]);
        let second = base64::engine::general_purpose::STANDARD.encode([3]);
        let ports = MemoryKernelPorts::default().with_stream_items(vec![
            Ok(stream_event(0, 1, &first)),
            Ok(stream_event(0, 2, &latest)),
            Ok(stream_event(1, 1, &second)),
        ]);
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-stream");

        runtime
            .submit_generation_work(stream_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let directive = runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        assert_eq!(directive, QueueDirective::Idle);
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Succeeded));
        let resources = ports.registered_resources();
        assert_eq!(
            resources["resource:job-stream:stream:0"].kind,
            ResourceKind::StreamFinalImage
        );
        assert_eq!(
            resources["resource:job-stream:stream:0"].bytes,
            vec![2, 2, 2]
        );
        assert_eq!(resources["resource:job-stream:stream:1"].bytes, vec![3]);
        let stream_artifact = &ports.artifacts()["artifact:job-stream:stream:0"];
        assert_eq!(stream_artifact.metadata.request_seed, Some(4242));
        assert_eq!(stream_artifact.metadata.seed, None);
        assert_eq!(
            stream_artifact.metadata.embedded_metadata_status,
            Some(atelier_artifacts::EmbeddedMetadataStatus::UnsupportedFormat)
        );
        let stream_events = ports
            .events()
            .into_iter()
            .filter(|event| matches!(event.kind, KernelEventKind::GenerationStreamChunk { .. }))
            .count();
        assert_eq!(stream_events, 3);
    });
}

#[test]
fn streaming_generation_error_uses_queue_failure_policy_without_persisting_frames() {
    block_on(async {
        let ports = MemoryKernelPorts::default().with_stream_items(vec![
            Ok(stream_event(0, 1, "QUJD")),
            Err(GenerationClientError::rate_limited(
                429,
                Some(Duration::from_secs(9)),
                "slow down",
            )),
        ]);
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-stream");

        runtime
            .submit_generation_work(stream_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let directive = runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        assert_eq!(
            directive,
            QueueDirective::Wait(QueueDelay::fixed(Duration::from_secs(9)))
        );
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::WaitingRetry));
        assert!(ports.registered_resources().is_empty());
        assert!(
            ports
                .events()
                .iter()
                .any(|event| matches!(event.kind, KernelEventKind::JobFailed { .. }))
        );
    });
}

#[test]
fn safety_failure_degrades_gallery_indexing_without_failing_job() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_generated_images(vec![generated_image(vec![1], None)])
            .failing_safety();
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Succeeded));
        assert!(ports.gallery_items().values().all(|item| matches!(
            &item.safety,
            atelier_gallery::GallerySafetyState::Failed {
                message,
                attempted_at_ms: 0
            } if message.contains("scanner unavailable")
        )));
        assert!(
            ports
                .events()
                .iter()
                .any(|event| matches!(event.kind, KernelEventKind::SafetyScanFailed { .. }))
        );
    });
}

#[test]
fn gallery_failure_marks_current_job_failed_and_returns_error() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_generated_images(vec![generated_image(vec![1], None)])
            .failing_gallery();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::Gallery(_)));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn resource_failure_marks_current_job_failed_and_returns_error() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_generated_images(vec![generated_image(vec![1], None)])
            .failing_resource();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::ResourceCatalog(_)));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn artifact_failure_marks_current_job_failed_and_returns_error() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_generated_images(vec![generated_image(vec![1], None)])
            .failing_artifact();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::Artifact(_)));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn streaming_generation_without_final_image_marks_job_failed() {
    block_on(async {
        let ports =
            MemoryKernelPorts::default().with_stream_items(vec![Ok(stream_event(0, 1, ""))]);
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-stream");

        runtime
            .submit_generation_work(stream_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let error = runtime
            .run_scheduled_generation_job(&job_id)
            .await
            .unwrap_err();

        assert!(matches!(error, KernelError::MissingGeneratedImage));
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::Failed));
    });
}

#[test]
fn novelai_rate_limit_retries_non_streaming_generation() {
    block_on(async {
        let ports = MemoryKernelPorts::default().failing_generate(
            GenerationClientError::rate_limited(429, Some(Duration::from_secs(4)), "slow down"),
        );
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let directive = runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        assert_eq!(
            directive,
            QueueDirective::Wait(QueueDelay::fixed(Duration::from_secs(4)))
        );
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::WaitingRetry));
        assert!(ports.events().into_iter().any(|event| matches!(
            event.kind,
            KernelEventKind::JobFailed {
                detail: Some(KernelFailureDetail::GenerationClient(
                    GenerationClientError::RateLimited {
                        status: 429,
                        retry_after: Some(delay),
                        ..
                    }
                )),
                ..
            } if delay == Duration::from_secs(4)
        )));
    });
}

#[test]
fn novelai_rate_limit_without_retry_after_uses_queue_fallback_delay() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .failing_generate(GenerationClientError::rate_limited(429, None, "slow down"));
        let mut runtime = KernelRuntime::with_retry_policy(
            ports,
            RetryPolicy {
                rate_limit_fallback: QueueDelay::range(
                    Duration::from_secs(41),
                    Duration::from_secs(43),
                ),
                ..RetryPolicy::default()
            },
        );
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();
        let directive = runtime.run_scheduled_generation_job(&job_id).await.unwrap();

        assert_eq!(
            directive,
            QueueDirective::Wait(QueueDelay::range(
                Duration::from_secs(41),
                Duration::from_secs(43)
            ))
        );
        assert_eq!(runtime.job_status(&job_id), Some(JobStatus::WaitingRetry));
    });
}

#[test]
fn pause_resume_and_stop_wrap_queue_directives() {
    block_on(async {
        let ports = MemoryKernelPorts::default();
        let mut runtime = KernelRuntime::new(ports);
        let job_id = JobId::new("job-1");

        runtime
            .submit_generation_work(image_work("batch-1", job_id.clone(), "1girl"))
            .await
            .unwrap();

        assert_eq!(runtime.pause().unwrap(), QueueDirective::Paused);
        assert_eq!(runtime.resume().unwrap(), QueueDirective::StartJob(job_id));
        assert_eq!(runtime.stop().unwrap(), QueueDirective::Idle);
    });
}

#[test]
fn active_stream_cancellation_invokes_the_stream_cancel_hook() {
    block_on(async {
        struct CancelNow;
        impl atelier_kernel::GenerationTaskCancellation for CancelNow {
            fn is_cancelled(&self) -> bool {
                true
            }
        }

        let ports = MemoryKernelPorts::default().with_pending_stream();
        let mut runtime = KernelRuntime::new(ports.clone());
        let job_id = JobId::new("job-cancel");
        runtime
            .submit_generation_work(stream_work("batch-cancel", job_id.clone(), "1girl"))
            .await
            .unwrap();

        let result = runtime
            .run_scheduled_generation_job_cancellable(&job_id, &CancelNow)
            .await;

        assert_eq!(
            result,
            Err(atelier_kernel::KernelError::GenerationCancelled)
        );
        assert!(ports.stream_cancelled());
    });
}

fn image_work(batch_id: &str, job_id: JobId, prompt: &str) -> SubmitGenerationWork {
    SubmitGenerationWork {
        batch_id: BatchId::new(batch_id),
        job_id,
        request: GenerationWorkRequest::Image(GenerateImageRequest {
            prompt: prompt.to_owned(),
            model: ImageModel::NaiDiffusion45Full,
            ..Default::default()
        }),
        context: GenerationPlanContext::default(),
    }
}

fn stream_work(batch_id: &str, job_id: JobId, prompt: &str) -> SubmitGenerationWork {
    SubmitGenerationWork {
        batch_id: BatchId::new(batch_id),
        job_id,
        request: GenerationWorkRequest::Stream(GenerateImageStreamRequest {
            base: GenerateImageRequest {
                prompt: prompt.to_owned(),
                model: ImageModel::NaiDiffusion45Full,
                ..Default::default()
            },
            ..Default::default()
        }),
        context: GenerationPlanContext::default(),
    }
}

fn stream_event(sample_index: u32, step_index: u32, image: &str) -> ImageStreamEvent {
    ImageStreamEvent {
        event_type: "step".to_owned(),
        sample_index,
        step_index: Some(step_index),
        generation_id: 7,
        sigma: None,
        image: image.to_owned(),
    }
}
