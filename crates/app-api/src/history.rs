use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::generation::QueueDirectiveDto;
use crate::resource::ResourceRefDto;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunHistoryKindDto {
    Generation,
    Director,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunHistoryStatusDto {
    Queued,
    Preparing,
    Running,
    Waiting,
    Paused,
    Succeeded,
    Failed,
    Skipped,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunHistoryQueryDto {
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<RunHistoryKindDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RunHistoryStatusDto>,
}

impl Default for RunHistoryQueryDto {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            kind: None,
            status: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RunHistoryOutputStateDto {
    Available,
    Deleted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunHistoryOutputDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_index: Option<u32>,
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<ResourceRefDto>,
    pub asset_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_kind: Option<String>,
    pub state: RunHistoryOutputStateDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunHistoryItemDto {
    pub run_id: String,
    pub kind: RunHistoryKindDto,
    pub status: RunHistoryStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    pub recoverable: bool,
    pub outputs: Vec<RunHistoryOutputDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RunHistoryPageDto {
    pub items: Vec<RunHistoryItemDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteRunHistoryItemsRequestDto {
    pub run_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteRunHistoryItemsResponseDto {
    pub deleted: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RerunGenerationHistoryItemRequestDto {
    pub run_id: String,
    pub batch_id: String,
    pub job_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RerunGenerationHistoryItemResponseDto {
    pub directive: QueueDirectiveDto,
    pub item: RunHistoryItemDto,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum GenerationBatchHistoryStatusDto {
    Queued,
    Preparing,
    Running,
    Waiting,
    Paused,
    Succeeded,
    PartiallySucceeded,
    Failed,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationHistoryQueryDto {
    pub offset: usize,
    pub limit: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<GenerationBatchHistoryStatusDto>,
}

impl Default for GenerationHistoryQueryDto {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            status: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationHistoryBatchDto {
    pub batch_id: String,
    pub status: GenerationBatchHistoryStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    pub request_count: usize,
    pub completed_request_count: usize,
    pub expected_sample_count: u32,
    pub completed_sample_count: usize,
    pub available_sample_count: usize,
    pub outputs: Vec<RunHistoryOutputDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationHistoryPageDto {
    pub items: Vec<GenerationHistoryBatchDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationHistoryBatchRequestDto {
    pub batch_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationHistoryRequestDto {
    pub run_id: String,
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_run_id: Option<String>,
    pub request_index: u32,
    pub expected_samples: u32,
    pub status: RunHistoryStatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    pub outputs: Vec<RunHistoryOutputDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GenerationHistoryBatchDetailDto {
    pub batch: GenerationHistoryBatchDto,
    pub requests: Vec<GenerationHistoryRequestDto>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteGenerationHistoryBatchesRequestDto {
    pub batch_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteGenerationHistoryBatchesResponseDto {
    pub deleted_requests: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RerunGenerationHistoryBatchRequestDto {
    pub source_batch_id: String,
    pub batch_id: String,
    pub job_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RerunGenerationHistoryBatchResponseDto {
    pub directive: QueueDirectiveDto,
    pub batch: GenerationHistoryBatchDto,
}
