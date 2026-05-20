use serde::{Deserialize, Serialize};

use crate::resource::ResourceRefDto;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppEventDto {
    pub sequence: u64,
    pub kind: AppEventKindDto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventsSinceRequestDto {
    pub sequence: u64,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppEventPageDto {
    pub items: Vec<AppEventDto>,
    pub next_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AppEventKindDto {
    BatchSubmitted {
        batch_id: String,
    },
    JobPreparing {
        batch_id: String,
        job_id: String,
    },
    PromptCompiled {
        batch_id: String,
        job_id: String,
        expanded_prompt: String,
    },
    GenerationPlanned {
        batch_id: String,
        job_id: String,
        output_mode: String,
    },
    GenerationStreamChunk {
        batch_id: String,
        job_id: String,
        event_type: String,
        sample_index: u32,
        step_index: Option<u32>,
        generation_id: u32,
        sigma: Option<f32>,
        image: String,
    },
    SamplePersisted {
        batch_id: String,
        job_id: String,
        sample_index: u32,
        resource: ResourceRefDto,
        artifact_id: String,
    },
    GalleryIndexed {
        batch_id: String,
        job_id: String,
        item_id: String,
    },
    SafetyScanFailed {
        batch_id: String,
        job_id: String,
        resource: ResourceRefDto,
        message: String,
    },
    DirectorSafetyScanFailed {
        run_id: String,
        resource: ResourceRefDto,
        message: String,
    },
    JobSucceeded {
        batch_id: String,
        job_id: String,
    },
    JobFailed {
        batch_id: String,
        job_id: String,
        message: String,
    },
}
