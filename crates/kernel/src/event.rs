use nai_atelier_artifacts::ArtifactId;
use nai_atelier_gallery::GalleryItemId;
use nai_atelier_generation::{GenerationOutputMode, ImageStreamEvent};
use nai_atelier_jobs::{BatchId, JobId};
use nai_atelier_resource_catalog::ResourceRef;

#[derive(Clone, Debug, PartialEq)]
pub struct KernelEvent {
    pub sequence: u64,
    pub kind: KernelEventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KernelEventKind {
    BatchSubmitted {
        batch_id: BatchId,
    },
    JobPreparing {
        batch_id: BatchId,
        job_id: JobId,
    },
    PromptCompiled {
        batch_id: BatchId,
        job_id: JobId,
        expanded_prompt: String,
    },
    GenerationPlanned {
        batch_id: BatchId,
        job_id: JobId,
        output_mode: GenerationOutputMode,
    },
    GenerationStreamChunk {
        batch_id: BatchId,
        job_id: JobId,
        event: ImageStreamEvent,
    },
    SamplePersisted {
        batch_id: BatchId,
        job_id: JobId,
        sample_index: u32,
        resource: ResourceRef,
        artifact_id: ArtifactId,
    },
    GalleryIndexed {
        batch_id: BatchId,
        job_id: JobId,
        item_id: GalleryItemId,
    },
    SafetyScanFailed {
        batch_id: BatchId,
        job_id: JobId,
        resource: ResourceRef,
        message: String,
    },
    JobSucceeded {
        batch_id: BatchId,
        job_id: JobId,
    },
    JobFailed {
        batch_id: BatchId,
        job_id: JobId,
        message: String,
    },
}
