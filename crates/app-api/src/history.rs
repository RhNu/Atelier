use serde::{Deserialize, Serialize};

use crate::generation::QueueDirectiveDto;
use crate::resource::ResourceRefDto;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunHistoryKindDto {
    Generation,
    Director,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunHistoryOutputDto {
    pub artifact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    pub resource: ResourceRefDto,
    pub asset_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunHistoryPageDto {
    pub items: Vec<RunHistoryItemDto>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerunGenerationHistoryItemRequestDto {
    pub run_id: String,
    pub batch_id: String,
    pub job_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerunGenerationHistoryItemResponseDto {
    pub directive: QueueDirectiveDto,
    pub item: RunHistoryItemDto,
}
