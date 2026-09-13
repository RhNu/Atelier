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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LibraryFolderDto, LibraryNamespaceDto, LibraryResourceDto};

    #[test]
    fn root_entries_serialize_nullable_parent_fields() {
        let folder = LibraryFolderDto {
            folder_id: "folder-id".to_owned(),
            namespace: LibraryNamespaceDto::PromptChunk,
            parent_id: None,
            identifier: "styles".to_owned(),
            display_name: "Styles".to_owned(),
            path: "styles".to_owned(),
            created_at_ms: 1,
            updated_at_ms: 1,
        };
        let resource = LibraryResourceDto {
            resource_id: "resource-id".to_owned(),
            namespace: LibraryNamespaceDto::PromptChunk,
            folder_id: None,
            identifier: "lighting".to_owned(),
            display_name: "Lighting".to_owned(),
            aliases: Vec::new(),
            path: "lighting".to_owned(),
            created_at_ms: 1,
            updated_at_ms: 1,
        };

        assert_eq!(
            serde_json::to_value(folder).unwrap()["parent_id"],
            json!(null)
        );
        assert_eq!(
            serde_json::to_value(resource).unwrap()["folder_id"],
            json!(null)
        );
    }
}
