use std::collections::BTreeMap;

use atelier_generation::{GenerateImageStreamRequest, GenerationRequestPlan};
use atelier_jobs::{JobId, QueueDirective};
use atelier_resource_catalog::ResourceKind;
use base64::Engine;
use futures_util::StreamExt;

use crate::workflow::generation::{
    PersistSample, fail_job, handle_novelai_failure, persist_sample,
};
use crate::{
    GenerationPayloadStore, KernelClock, KernelError, KernelEventKind, KernelEventSink,
    KernelGenerationPorts, KernelResult, KernelRuntime,
};

pub async fn run_stream_generation<P>(
    runtime: &mut KernelRuntime<P>,
    batch_id: &atelier_jobs::BatchId,
    job_id: &JobId,
    prepared_payload_ref: &atelier_jobs::JobPayloadRef,
    prompt_snapshot: &str,
    plan: &GenerationRequestPlan,
    request: GenerateImageStreamRequest,
) -> KernelResult<QueueDirective>
where
    P: GenerationPayloadStore + KernelClock + KernelEventSink + KernelGenerationPorts,
{
    let mut stream = match runtime.ports_ref().generate_stream(request).await {
        Ok(stream) => stream,
        Err(error) => return handle_novelai_failure(runtime, batch_id, job_id, error).await,
    };
    let mut latest_images = BTreeMap::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(event) => {
                if !event.image.trim().is_empty() {
                    latest_images.insert(event.sample_index, event.image.clone());
                }
                runtime
                    .emit(KernelEventKind::GenerationStreamChunk {
                        batch_id: batch_id.clone(),
                        job_id: job_id.clone(),
                        event,
                    })
                    .await;
            }
            Err(error) => return handle_novelai_failure(runtime, batch_id, job_id, error).await,
        }
    }
    if latest_images.is_empty() {
        fail_job(runtime, batch_id, job_id, "stream ended without image data").await?;
        return Err(KernelError::MissingGeneratedImage);
    }
    for (sample_index, image) in latest_images {
        let bytes = match decode_stream_image(sample_index, &image) {
            Ok(bytes) => bytes,
            Err(error) => {
                fail_job(runtime, batch_id, job_id, &error.to_string()).await?;
                return Err(error);
            }
        };
        if let Err(error) = persist_sample(
            runtime,
            PersistSample {
                batch_id,
                job_id,
                prepared_payload_ref,
                prompt_snapshot,
                plan,
                sample_index,
                kind: ResourceKind::StreamFinalImage,
                id_segment: "stream",
                bytes,
                seed: None,
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

fn decode_stream_image(sample_index: u32, image: &str) -> KernelResult<Vec<u8>> {
    let trimmed = image.trim();
    let payload = trimmed
        .strip_prefix("data:")
        .and_then(|value| value.split_once(',').map(|(_, payload)| payload))
        .unwrap_or(trimmed);
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|error| KernelError::InvalidStreamImage {
            sample_index,
            message: error.to_string(),
        })
}
