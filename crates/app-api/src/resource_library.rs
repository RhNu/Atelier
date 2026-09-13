use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LibraryNamespaceDto {
    PromptChunk,
    MainPreset,
    CharacterPreset,
    Vibe,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LibraryFolderDto {
    pub folder_id: String,
    pub namespace: LibraryNamespaceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub identifier: String,
    pub display_name: String,
    pub path: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LibraryResourceDto {
    pub resource_id: String,
    pub namespace: LibraryNamespaceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub identifier: String,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub path: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LibrarySnapshotDto {
    pub folders: Vec<LibraryFolderDto>,
    pub resources: Vec<LibraryResourceDto>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GetLibrarySnapshotRequestDto {
    pub namespace: LibraryNamespaceDto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct UpsertLibraryFolderRequestDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub namespace: LibraryNamespaceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    pub identifier: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct UpdateLibraryResourceRequestDto {
    pub resource_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<String>,
    pub identifier: String,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteLibraryFolderRequestDto {
    pub folder_id: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DeleteLibraryFolderResponseDto {
    pub deleted: bool,
}
