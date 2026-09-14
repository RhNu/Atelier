use atelier_agent::{AgentError, AgentResult};
use atelier_app_api::{
    generation::{ImageModelDto, QualityPresetDto, UcPresetDto},
    prompt::{PromptChunkDto, PromptPresetBehaviorDto, PromptPresetDto},
};
use serde::{Deserialize, Serialize};

use super::{patch::Patch, text_edit::TextEdit};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Chunk,
    Preset,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTarget {
    pub kind: ResourceKind,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "resource", rename_all = "snake_case")]
pub enum ResourceDocument {
    Chunk(PromptChunkDto),
    Preset(PromptPresetDto),
}

impl ResourceDocument {
    pub fn target(&self) -> ResourceTarget {
        match self {
            Self::Chunk(value) => ResourceTarget {
                kind: ResourceKind::Chunk,
                id: value.chunk_id.clone(),
            },
            Self::Preset(value) => ResourceTarget {
                kind: ResourceKind::Preset,
                id: value.preset_id.clone(),
            },
        }
    }

    pub fn edit(&self, operations: &[ResourceOperation]) -> AgentResult<Self> {
        if operations.is_empty() {
            return Err(AgentError::validation(
                "at least one resource operation is required",
            ));
        }
        let mut result = self.clone();
        for operation in operations {
            operation.apply(&mut result)?;
        }
        Ok(result)
    }
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceOperation {
    EditText {
        field: ResourceTextField,
        edit: TextEdit,
    },
    SetMetadata {
        patch: MetadataPatch,
    },
    SetBehavior {
        field: BehaviorField,
        behavior: PromptPresetBehaviorDto,
    },
    SetOverrides {
        patch: OverridePatch,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceTextField {
    Content,
    PromptBefore,
    PromptAfter,
    PromptText,
    UcBefore,
    UcAfter,
    UcText,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorField {
    Prompt,
    NegativePrompt,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetadataPatch {
    path: Option<String>,
    folder_id: Patch<Option<String>>,
    display_name: Option<String>,
    aliases: Option<Vec<String>>,
    description: Patch<Option<String>>,
    models: Option<Vec<ImageModelDto>>,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OverridePatch {
    quality: Patch<Option<QualityPresetDto>>,
    uc_preset: Patch<Option<UcPresetDto>>,
}

impl ResourceOperation {
    fn apply(&self, resource: &mut ResourceDocument) -> AgentResult<()> {
        match self {
            Self::EditText { field, edit } => {
                let target = text_field(resource, field)?;
                *target = edit.apply(target)?;
            }
            Self::SetMetadata { patch } => patch.apply(resource),
            Self::SetBehavior { field, behavior } => {
                let ResourceDocument::Preset(preset) = resource else {
                    return Err(AgentError::validation("behavior applies only to presets"));
                };
                match field {
                    BehaviorField::Prompt => preset.prompt_behavior = behavior.clone(),
                    BehaviorField::NegativePrompt => preset.uc_behavior = behavior.clone(),
                }
            }
            Self::SetOverrides { patch } => {
                let ResourceDocument::Preset(preset) = resource else {
                    return Err(AgentError::validation("overrides apply only to presets"));
                };
                patch.quality.apply(&mut preset.quality_override);
                if let Patch::Set(value) = patch.uc_preset {
                    preset.uc_preset_override = value.map(super::resource_input::uc_preset_name);
                }
            }
        }
        Ok(())
    }
}

fn text_field<'a>(
    resource: &'a mut ResourceDocument,
    field: &ResourceTextField,
) -> AgentResult<&'a mut String> {
    if let ResourceDocument::Chunk(chunk) = resource {
        return match field {
            ResourceTextField::Content => Ok(&mut chunk.content),
            _ => Err(AgentError::validation("chunk text field must be content")),
        };
    }
    let ResourceDocument::Preset(preset) = resource else {
        unreachable!()
    };
    let behavior = match field {
        ResourceTextField::PromptBefore
        | ResourceTextField::PromptAfter
        | ResourceTextField::PromptText => &mut preset.prompt_behavior,
        ResourceTextField::UcBefore | ResourceTextField::UcAfter | ResourceTextField::UcText => {
            &mut preset.uc_behavior
        }
        ResourceTextField::Content => {
            return Err(AgentError::validation("preset has no content field"));
        }
    };
    match (behavior, field) {
        (
            PromptPresetBehaviorDto::Surround { before, .. },
            ResourceTextField::PromptBefore | ResourceTextField::UcBefore,
        ) => Ok(before),
        (
            PromptPresetBehaviorDto::Surround { after, .. },
            ResourceTextField::PromptAfter | ResourceTextField::UcAfter,
        ) => Ok(after),
        (
            PromptPresetBehaviorDto::Replace { text },
            ResourceTextField::PromptText | ResourceTextField::UcText,
        ) => Ok(text),
        _ => Err(AgentError::validation(
            "text field does not match the current preset behavior; use set_behavior to change its mode",
        )),
    }
}

impl MetadataPatch {
    fn apply(&self, resource: &mut ResourceDocument) {
        let (path, folder_id, display_name, aliases, description, models) = match resource {
            ResourceDocument::Chunk(value) => (
                &mut value.path,
                &mut value.folder_id,
                &mut value.display_name,
                &mut value.aliases,
                &mut value.description,
                &mut value.models,
            ),
            ResourceDocument::Preset(value) => (
                &mut value.path,
                &mut value.folder_id,
                &mut value.display_name,
                &mut value.aliases,
                &mut value.description,
                &mut value.models,
            ),
        };
        if let Some(value) = &self.path {
            path.clone_from(value);
        }
        self.folder_id.apply(folder_id);
        if let Some(value) = &self.display_name {
            display_name.clone_from(value);
        }
        if let Some(value) = &self.aliases {
            aliases.clone_from(value);
        }
        self.description.apply(description);
        if let Some(value) = &self.models {
            models.clone_from(value);
        }
    }
}

#[cfg(test)]
mod tests;
