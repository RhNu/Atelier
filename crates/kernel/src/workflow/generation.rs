use nai_atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactReplayManifest, ArtifactSource,
    RegisterArtifactRequest, VisualAssetRef, VisualAssetRole,
};
use nai_atelier_generation::{
    GenerateImageStreamRequest, GenerationOutputMode, GenerationRequestPlan, SeedMode,
    plan_generation_request, plan_generation_stream_request,
};
use nai_atelier_jobs::{JobFailureImpact, JobId, QueueDirective};
use nai_atelier_prompt_resources::CompilePromptRequest;
use nai_atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceId, ResourceKind, ResourceLifecycle,
    ResourceOwner, ResourceOwnerKind, ResourceRelation, ResourceVariantKind,
};

use crate::runtime::{prepared_payload_ref, submitted_payload_ref};
use crate::{
    GenerationPayloadStore, GenerationWorkRequest, KernelClock, KernelError, KernelEventKind,
    KernelEventSink, KernelGenerationPorts, KernelResult, KernelRuntime, PreparedGenerationPayload,
};

pub async fn run_scheduled_generation_job<P>(
    runtime: &mut KernelRuntime<P>,
    job_id: &JobId,
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

    let compiled = match runtime
        .ports_ref()
        .compile_prompt(CompilePromptRequest::new(submitted.request.prompt()))
        .await
    {
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
            expanded_prompt: compiled.expanded_prompt.clone(),
        })
        .await;

    let request = submitted
        .request
        .clone()
        .with_prompt(compiled.expanded_prompt.clone());
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
            compiled_prompt: compiled.clone(),
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
                &compiled.expanded_prompt,
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
                &compiled.expanded_prompt,
                &plan,
                request,
            )
            .await
        }
    }
}

fn plan_request(
    request: GenerationWorkRequest,
    context: nai_atelier_generation::GenerationPlanContext,
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
    batch_id: &nai_atelier_jobs::BatchId,
    job_id: &JobId,
    prepared_payload_ref: &nai_atelier_jobs::JobPayloadRef,
    prompt_snapshot: &str,
    plan: &GenerationRequestPlan,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let images = match runtime
        .ports_ref()
        .generate(plan.normalized_request.clone())
        .await
    {
        Ok(images) => images,
        Err(error) => return handle_novelai_failure(runtime, batch_id, job_id, error).await,
    };
    if images.is_empty() {
        fail_job(runtime, batch_id, job_id, "generation returned no images").await?;
        return Err(KernelError::MissingGeneratedImage);
    }
    for (index, image) in images.into_iter().enumerate() {
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
                seed: image.seed,
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
    pub batch_id: &'a nai_atelier_jobs::BatchId,
    pub job_id: &'a JobId,
    pub prepared_payload_ref: &'a nai_atelier_jobs::JobPayloadRef,
    pub prompt_snapshot: &'a str,
    pub plan: &'a GenerationRequestPlan,
    pub sample_index: u32,
    pub kind: ResourceKind,
    pub id_segment: &'a str,
    pub bytes: Vec<u8>,
    pub seed: Option<i64>,
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
            metadata: artifact_metadata(sample.plan, sample.sample_index, sample.seed),
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
    let safety = match runtime.ports_ref().score_image(resource.clone()).await {
        Ok(assessment) => assessment,
        Err(error) => {
            runtime
                .emit(KernelEventKind::SafetyScanFailed {
                    batch_id: sample.batch_id.clone(),
                    job_id: sample.job_id.clone(),
                    resource: resource.clone(),
                    message: error.to_string(),
                })
                .await;
            None
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
            artifact_id,
        })
        .await;
    runtime
        .emit(KernelEventKind::GalleryIndexed {
            batch_id: sample.batch_id.clone(),
            job_id: sample.job_id.clone(),
            item_id: item.id,
        })
        .await;
    Ok(())
}

fn artifact_metadata(
    plan: &GenerationRequestPlan,
    sample_index: u32,
    seed: Option<i64>,
) -> ArtifactMetadata {
    ArtifactMetadata {
        seed: seed.or(match plan.seed_mode {
            SeedMode::Fixed(seed) => Some(seed),
            SeedMode::Auto => None,
        }),
        sample_index: Some(sample_index),
        model_name: Some(plan.normalized_request.model.as_str().to_owned()),
        extensions: std::collections::BTreeMap::default(),
    }
}

pub async fn handle_novelai_failure<P>(
    runtime: &mut KernelRuntime<P>,
    batch_id: &nai_atelier_jobs::BatchId,
    job_id: &JobId,
    error: nai_atelier_foundation::NovelAiError,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let directive = runtime.mark_failed(job_id, JobFailureImpact::from_novelai_error(&error))?;
    runtime
        .emit(KernelEventKind::JobFailed {
            batch_id: batch_id.clone(),
            job_id: job_id.clone(),
            message: error.to_string(),
        })
        .await;
    Ok(directive)
}

pub async fn fail_job<P>(
    runtime: &mut KernelRuntime<P>,
    batch_id: &nai_atelier_jobs::BatchId,
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
        })
        .await;
    Ok(directive)
}
