use atelier_app_api::{
    generation::{ImageModelDto, QualityPresetDto, UcPresetDto},
    prompt::{
        PromptPresetBehaviorDto, PromptPresetKindDto, UpsertPromptChunkRequestDto,
        UpsertPromptPresetRequestDto,
    },
};
use serde::Deserialize;

use super::resource_edit::ResourceDocument;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceMetadata {
    pub path: String,
    pub folder_id: Option<String>,
    pub display_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub models: Vec<ImageModelDto>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CreateResource {
    Chunk {
        metadata: ResourceMetadata,
        content: String,
    },
    Preset {
        metadata: ResourceMetadata,
        preset_kind: PromptPresetKindDto,
        prompt_behavior: PromptPresetBehaviorDto,
        uc_behavior: PromptPresetBehaviorDto,
        quality_override: Option<QualityPresetDto>,
        uc_preset_override: Option<UcPresetDto>,
    },
}

pub enum ResourceWrite {
    Chunk(UpsertPromptChunkRequestDto),
    Preset(UpsertPromptPresetRequestDto),
}

impl From<CreateResource> for ResourceWrite {
    fn from(value: CreateResource) -> Self {
        match value {
            CreateResource::Chunk { metadata, content } => {
                Self::Chunk(UpsertPromptChunkRequestDto {
                    chunk_id: None,
                    path: metadata.path,
                    folder_id: metadata.folder_id,
                    display_name: metadata.display_name,
                    aliases: metadata.aliases,
                    content,
                    description: metadata.description,
                    preview: None,
                    models: metadata.models,
                })
            }
            CreateResource::Preset {
                metadata,
                preset_kind,
                prompt_behavior,
                uc_behavior,
                quality_override,
                uc_preset_override,
            } => Self::Preset(UpsertPromptPresetRequestDto {
                preset_id: None,
                kind: preset_kind,
                path: metadata.path,
                folder_id: metadata.folder_id,
                display_name: metadata.display_name,
                aliases: metadata.aliases,
                description: metadata.description,
                prompt_behavior,
                uc_behavior,
                quality_override,
                uc_preset_override: uc_preset_override.map(uc_preset_name),
                preview: None,
                models: metadata.models,
            }),
        }
    }
}

impl From<ResourceDocument> for ResourceWrite {
    fn from(value: ResourceDocument) -> Self {
        match value {
            ResourceDocument::Chunk(value) => Self::Chunk(UpsertPromptChunkRequestDto {
                chunk_id: Some(value.chunk_id),
                path: value.path,
                folder_id: value.folder_id,
                display_name: value.display_name,
                aliases: value.aliases,
                content: value.content,
                description: value.description,
                preview: value.preview,
                models: value.models,
            }),
            ResourceDocument::Preset(value) => Self::Preset(UpsertPromptPresetRequestDto {
                preset_id: Some(value.preset_id),
                kind: value.kind,
                path: value.path,
                folder_id: value.folder_id,
                display_name: value.display_name,
                aliases: value.aliases,
                description: value.description,
                prompt_behavior: value.prompt_behavior,
                uc_behavior: value.uc_behavior,
                quality_override: value.quality_override,
                uc_preset_override: value.uc_preset_override,
                preview: value.preview,
                models: value.models,
            }),
        }
    }
}

impl ResourceWrite {
    pub fn copy_to(&mut self, path: String, folder_id: Option<String>, display_name: String) {
        match self {
            Self::Chunk(value) => {
                value.chunk_id = None;
                value.path = path;
                value.folder_id = folder_id;
                value.display_name = display_name;
            }
            Self::Preset(value) => {
                value.preset_id = None;
                value.path = path;
                value.folder_id = folder_id;
                value.display_name = display_name;
            }
        }
    }
}

pub fn uc_preset_name(value: UcPresetDto) -> String {
    serde_json::to_value(value)
        .expect("enum serializes")
        .as_str()
        .expect("string enum")
        .to_owned()
}
