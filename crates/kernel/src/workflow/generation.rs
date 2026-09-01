use atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactReplayManifest, ArtifactSource,
    EmbeddedMetadataStatus, EmbeddedMetadataWarning, RegisterArtifactRequest, VisualAssetRef,
    VisualAssetRole,
};
use atelier_gallery::GallerySafetyState;
use atelier_generation::{
    GenerateImageStreamRequest, GeneratedImageMetadata, GeneratedImageMetadataWarning,
    GenerationClientError, GenerationOutputMode, GenerationRequestPlan, ImageModel,
    plan_generation_request, plan_generation_stream_request,
};
use atelier_jobs::{JobFailureImpact, JobId, QueueDelay, QueueDirective, RetryPolicy};
use atelier_prompt_resources::{CompilePromptRequest, PromptResourceResult};
use atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceId, ResourceKind, ResourceLifecycle,
    ResourceOwner, ResourceOwnerKind, ResourceRelation, ResourceVariantKind,
};

use crate::runtime::{prepared_payload_ref, submitted_payload_ref};
use crate::{
    CompiledGenerationCharacterPrompts, CompiledGenerationPrompts, GenerationPayloadStore,
    GenerationWorkRequest, KernelClock, KernelError, KernelEventKind, KernelEventSink,
    KernelFailureDetail, KernelGenerationPorts, KernelResult, KernelRuntime,
    PreparedGenerationPayload,
};

pub async fn run_scheduled_generation_job<P>(
    runtime: &mut KernelRuntime<P>,
    job_id: &JobId,
    cancellation: &dyn crate::GenerationTaskCancellation,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let submitted_ref = submitted_payload_ref(job_id);
    let submitted = runtime
        .ports_ref()
        .get_submitted_payload(&submitted_ref)
        .await?
        .ok_or_else(|| KernelError::MissingSubmittedPayload(submitted_ref.clone()))?;
    runtime.mark_preparing(job_id)?;
    runtime
        .emit(KernelEventKind::JobPreparing {
            batch_id: submitted.batch_id.clone(),
            job_id: job_id.clone(),
        })
        .await;

    let compiled = match compile_generation_prompts(runtime.ports_ref(), &submitted.request).await {
        Ok(compiled) => compiled,
        Err(error) => {
            let error = KernelError::from(error);
            fail_job(runtime, &submitted.batch_id, job_id, &error.to_string()).await?;
            return Err(error);
        }
    };
    runtime
        .emit(KernelEventKind::PromptCompiled {
            batch_id: submitted.batch_id.clone(),
            job_id: job_id.clone(),
            expanded_prompt: compiled.prompt.expanded_prompt.clone(),
        })
        .await;

    let request = submitted.request.clone().with_compiled_prompts(&compiled);
    let plan = match plan_request(request.clone(), submitted.context) {
        Ok(plan) => plan,
        Err(error) => {
            fail_job(runtime, &submitted.batch_id, job_id, &error.to_string()).await?;
            return Err(error);
        }
    };
    runtime
        .emit(KernelEventKind::GenerationPlanned {
            batch_id: submitted.batch_id.clone(),
            job_id: job_id.clone(),
            output_mode: plan.output_mode,
        })
        .await;

    let prepared_ref = prepared_payload_ref(job_id);
    if let Err(error) = runtime
        .ports_ref()
        .save_prepared_payload(PreparedGenerationPayload {
            payload_ref: prepared_ref.clone(),
            submitted_payload_ref: submitted_ref,
            batch_id: submitted.batch_id.clone(),
            job_id: job_id.clone(),
            request,
            compiled_prompt: compiled.prompt.clone(),
            plan: plan.clone(),
        })
        .await
    {
        fail_job(runtime, &submitted.batch_id, job_id, &error.to_string()).await?;
        return Err(error);
    }
    runtime.mark_running(job_id, prepared_ref.clone())?;

    match plan.output_mode {
        GenerationOutputMode::Image => {
            run_image_generation(
                runtime,
                &submitted.batch_id,
                job_id,
                &prepared_ref,
                &compiled.prompt.expanded_prompt,
                &plan,
            )
            .await
        }
        GenerationOutputMode::Stream(stream) => {
            let request = GenerateImageStreamRequest {
                base: plan.normalized_request.clone(),
                stream,
            };
            crate::workflow::stream::run_stream_generation(
                runtime,
                &submitted.batch_id,
                job_id,
                &prepared_ref,
                &compiled.prompt.expanded_prompt,
                &plan,
                request,
                cancellation,
            )
            .await
        }
    }
}

async fn compile_generation_prompts<P>(
    ports: &P,
    request: &GenerationWorkRequest,
) -> PromptResourceResult<CompiledGenerationPrompts>
where
    P: KernelGenerationPorts,
{
    let model = request.model();
    let prompt = ports
        .compile_prompt(CompilePromptRequest::new(request.prompt(), model))
        .await?;
    let negative_prompt = compile_optional_prompt(ports, request.negative_prompt(), model).await?;
    let mut characters = Vec::new();
    if let Some(request_characters) = request.characters() {
        characters.reserve(request_characters.len());
        for character in request_characters {
            characters.push(CompiledGenerationCharacterPrompts {
                prompt: compile_optional_prompt(ports, Some(character.prompt.as_str()), model)
                    .await?,
                negative_prompt: compile_optional_prompt(
                    ports,
                    character.negative_prompt.as_deref(),
                    model,
                )
                .await?,
            });
        }
    }
    Ok(CompiledGenerationPrompts {
        prompt,
        negative_prompt,
        characters,
    })
}

async fn compile_optional_prompt<P>(
    ports: &P,
    prompt: Option<&str>,
    model: ImageModel,
) -> PromptResourceResult<Option<atelier_prompt_resources::CompiledPrompt>>
where
    P: KernelGenerationPorts,
{
    let Some(prompt) = prompt else {
        return Ok(None);
    };
    if prompt.trim().is_empty() {
        return Ok(None);
    }
    ports
        .compile_prompt(CompilePromptRequest::new(prompt, model))
        .await
        .map(Some)
}

fn plan_request(
    request: GenerationWorkRequest,
    context: atelier_generation::GenerationPlanContext,
) -> Result<GenerationRequestPlan, KernelError> {
    match request {
        GenerationWorkRequest::Image(request) => {
            plan_generation_request(request, context).map_err(KernelError::from)
        }
        GenerationWorkRequest::Stream(request) => {
            plan_generation_stream_request(request, context).map_err(KernelError::from)
        }
    }
}

async fn run_image_generation<P>(
    runtime: &mut KernelRuntime<P>,
    batch_id: &atelier_jobs::BatchId,
    job_id: &JobId,
    prepared_payload_ref: &atelier_jobs::JobPayloadRef,
    prompt_snapshot: &str,
    plan: &GenerationRequestPlan,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let result = match runtime
        .ports_ref()
        .generate(plan.normalized_request.clone())
        .await
    {
        Ok(result) => result,
        Err(error) => return handle_novelai_failure(runtime, batch_id, job_id, error).await,
    };
    if result.images.is_empty() {
        fail_job(runtime, batch_id, job_id, "generation returned no images").await?;
        return Err(KernelError::MissingGeneratedImage);
    }
    for (index, image) in result.images.into_iter().enumerate() {
        if let Err(error) = persist_sample(
            runtime,
            PersistSample {
                batch_id,
                job_id,
                prepared_payload_ref,
                prompt_snapshot,
                plan,
                sample_index: u32::try_from(index).unwrap_or(u32::MAX),
                kind: ResourceKind::GeneratedImage,
                id_segment: "sample",
                bytes: image.bytes,
                request_seed: result.resolved_seed,
                metadata: image.metadata,
            },
        )
        .await
        {
            fail_job(runtime, batch_id, job_id, &error.to_string()).await?;
            return Err(error);
        }
    }
    let directive = runtime.mark_succeeded(job_id)?;
    runtime
        .emit(KernelEventKind::JobSucceeded {
            batch_id: batch_id.clone(),
            job_id: job_id.clone(),
        })
        .await;
    Ok(directive)
}

pub struct PersistSample<'a> {
    pub batch_id: &'a atelier_jobs::BatchId,
    pub job_id: &'a JobId,
    pub prepared_payload_ref: &'a atelier_jobs::JobPayloadRef,
    pub prompt_snapshot: &'a str,
    pub plan: &'a GenerationRequestPlan,
    pub sample_index: u32,
    pub kind: ResourceKind,
    pub id_segment: &'a str,
    pub bytes: Vec<u8>,
    pub request_seed: i64,
    pub metadata: GeneratedImageMetadata,
}

pub async fn persist_sample<P>(
    runtime: &mut KernelRuntime<P>,
    sample: PersistSample<'_>,
) -> KernelResult<()>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let resource_id = ResourceId::new(format!(
        "resource:{}:{}:{}",
        sample.job_id.as_str(),
        sample.id_segment,
        sample.sample_index
    ));
    let resource = runtime
        .ports_ref()
        .register_resource(RegisterResourceRequest {
            resource_id,
            kind: sample.kind,
            lifecycle: ResourceLifecycle::JobScoped,
            owner: ResourceOwner::new(ResourceOwnerKind::Job, sample.job_id.as_str()),
            relation: ResourceRelation::Primary,
            blob: BlobWriteIntent::Bytes(sample.bytes),
        })
        .await?;
    let artifact_id = ArtifactId::new(format!(
        "artifact:{}:{}:{}",
        sample.job_id.as_str(),
        sample.id_segment,
        sample.sample_index
    ));
    let artifact = runtime
        .ports_ref()
        .register_artifact(RegisterArtifactRequest {
            id: artifact_id.clone(),
            kind: ArtifactKind::GeneratedImage,
            source: ArtifactSource::GenerationJob {
                job_id: sample.job_id.as_str().to_owned(),
                batch_id: Some(sample.batch_id.as_str().to_owned()),
            },
            primary_resource: resource.clone(),
            metadata: artifact_metadata(
                sample.plan,
                sample.sample_index,
                sample.request_seed,
                &sample.metadata,
            ),
            replay: Some(ArtifactReplayManifest {
                payload_ref: Some(submitted_payload_ref(sample.job_id).as_str().to_owned()),
                prepared_payload_ref: Some(sample.prepared_payload_ref.as_str().to_owned()),
                prompt_snapshot: Some(sample.prompt_snapshot.to_owned()),
                negative_prompt_snapshot: sample.plan.normalized_request.negative_prompt.clone(),
            }),
            assets: vec![VisualAssetRef {
                role: VisualAssetRole::Original,
                resource: resource.clone(),
                variant_kind: Some(ResourceVariantKind::Original),
            }],
        })
        .await?;
    let attempted_at_ms = runtime.ports_ref().now_ms();
    let safety = match runtime.ports_ref().score_image(resource.clone()).await {
        Ok(Some(assessment)) => GallerySafetyState::Scanned(Box::new(assessment)),
        Ok(None) => GallerySafetyState::Unavailable {
            message: "automatic safety scanning is unavailable".to_owned(),
        },
        Err(error) => {
            runtime
                .emit(KernelEventKind::SafetyScanFailed {
                    batch_id: sample.batch_id.clone(),
                    job_id: sample.job_id.clone(),
                    resource: resource.clone(),
                    message: error.to_string(),
                })
                .await;
            GallerySafetyState::Failed {
                message: error.to_string(),
                attempted_at_ms,
            }
        }
    };
    let item = runtime
        .ports_ref()
        .index_gallery_item(artifact, runtime.ports_ref().now_ms(), safety)
        .await?;
    runtime
        .emit(KernelEventKind::SamplePersisted {
            batch_id: sample.batch_id.clone(),
            job_id: sample.job_id.clone(),
            sample_index: sample.sample_index,
            resource,
            artifact_id: artifact_id.clone(),
        })
        .await;
    runtime
        .emit(KernelEventKind::GalleryIndexed {
            batch_id: sample.batch_id.clone(),
            job_id: sample.job_id.clone(),
            sample_index: sample.sample_index,
            artifact_id,
            item_id: item.id,
        })
        .await;
    Ok(())
}

fn artifact_metadata(
    plan: &GenerationRequestPlan,
    sample_index: u32,
    request_seed: i64,
    embedded: &GeneratedImageMetadata,
) -> ArtifactMetadata {
    let (seed, status, prompt, negative_prompt, metadata_json, error, warnings) = match embedded {
        GeneratedImageMetadata::Parsed(metadata) => (
            metadata.seed,
            EmbeddedMetadataStatus::Parsed,
            metadata.prompt.clone(),
            metadata.negative_prompt.clone(),
            Some(metadata.metadata_json.clone()),
            None,
            metadata
                .warnings
                .iter()
                .map(|warning| match warning {
                    GeneratedImageMetadataWarning::InvalidCommentJson => {
                        EmbeddedMetadataWarning::InvalidCommentJson
                    }
                    GeneratedImageMetadataWarning::InvalidTextChunk { keyword, message } => {
                        EmbeddedMetadataWarning::InvalidTextChunk {
                            keyword: keyword.clone(),
                            message: message.clone(),
                        }
                    }
                })
                .collect(),
        ),
        GeneratedImageMetadata::NotPresent => (
            None,
            EmbeddedMetadataStatus::NotPresent,
            None,
            None,
            None,
            None,
            Vec::new(),
        ),
        GeneratedImageMetadata::UnsupportedFormat => (
            None,
            EmbeddedMetadataStatus::UnsupportedFormat,
            None,
            None,
            None,
            None,
            Vec::new(),
        ),
        GeneratedImageMetadata::Invalid { message } => (
            None,
            EmbeddedMetadataStatus::Invalid,
            None,
            None,
            None,
            Some(message.clone()),
            Vec::new(),
        ),
    };
    ArtifactMetadata {
        request_seed: Some(request_seed),
        seed,
        sample_index: Some(sample_index),
        model_name: Some(plan.normalized_request.model.as_str().to_owned()),
        embedded_metadata_status: Some(status),
        embedded_prompt: prompt,
        embedded_negative_prompt: negative_prompt,
        embedded_metadata_json: metadata_json,
        embedded_metadata_error: error,
        embedded_metadata_warnings: warnings,
        extensions: std::collections::BTreeMap::default(),
    }
}

pub async fn handle_novelai_failure<P>(
    runtime: &mut KernelRuntime<P>,
    batch_id: &atelier_jobs::BatchId,
    job_id: &JobId,
    error: GenerationClientError,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let impact = generation_failure_impact(&error, runtime.retry_policy());
    let directive = runtime.mark_failed(job_id, impact)?;
    runtime
        .emit(KernelEventKind::JobFailed {
            batch_id: batch_id.clone(),
            job_id: job_id.clone(),
            message: error.to_string(),
            detail: Some(KernelFailureDetail::GenerationClient(error)),
        })
        .await;
    Ok(directive)
}

fn generation_failure_impact(
    error: &GenerationClientError,
    retry_policy: RetryPolicy,
) -> JobFailureImpact {
    match error {
        GenerationClientError::RateLimited { retry_after, .. } => {
            let delay =
                retry_after.map_or_else(|| retry_policy.rate_limit_fallback, QueueDelay::fixed);
            JobFailureImpact::RetryAfter(delay)
        }
        GenerationClientError::InvalidRequest { .. } => JobFailureImpact::FailCurrentAndContinue,
        GenerationClientError::Credential { .. }
        | GenerationClientError::Authentication { .. }
        | GenerationClientError::InsufficientCredit { .. }
        | GenerationClientError::RequestConflict { .. }
        | GenerationClientError::ServiceUnavailable { .. }
        | GenerationClientError::Transport { .. }
        | GenerationClientError::Decode { .. }
        | GenerationClientError::Metadata { .. }
        | GenerationClientError::UnknownApi { .. } => JobFailureImpact::PauseAndRetryCurrent,
    }
}

pub async fn fail_job<P>(
    runtime: &mut KernelRuntime<P>,
    batch_id: &atelier_jobs::BatchId,
    job_id: &JobId,
    message: &str,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let directive = runtime.mark_failed(job_id, JobFailureImpact::FailCurrentAndContinue)?;
    runtime
        .emit(KernelEventKind::JobFailed {
            batch_id: batch_id.clone(),
            job_id: job_id.clone(),
            message: message.to_owned(),
            detail: None,
        })
        .await;
    Ok(directive)
}
