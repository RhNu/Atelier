use atelier_agent::{AgentError, AgentResult};
use atelier_app_api::generation::{
    CharacterPositionDto, GenerationDraftCharacterDto, GenerationDraftCharacterPositionModeDto,
    GenerationDraftDto, GenerationDraftPromptStateDto, GenerationDraftSeedModeDto, ImageFormatDto,
    ImageModelDto, ImageSizeDto, NoiseScheduleDto, QualityPresetDto, SamplerDto, UcPresetDto,
};
use serde::Deserialize;

use super::patch::Patch;
use super::text_edit::TextEdit;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftEdit {
    pub operations: Vec<DraftOperation>,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum DraftOperation {
    EditText {
        target: PromptTarget,
        edit: TextEdit,
    },
    CreateCharacter {
        id: String,
        prompt: String,
    },
    UpdateCharacter {
        id: String,
        patch: CharacterPatch,
    },
    CopyCharacter {
        id: String,
        new_id: String,
    },
    RemoveCharacter {
        id: String,
    },
    ReorderCharacters {
        ids: Vec<String>,
    },
    SelectPreset {
        character_id: Option<String>,
        #[serde(deserialize_with = "Option::deserialize")]
        preset_id: Option<String>,
    },
    SetParameters {
        patch: ParameterPatch,
    },
    SelectModel {
        model: ImageModelDto,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptTarget {
    pub character_id: Option<String>,
    pub field: PromptField,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptField {
    Prompt,
    NegativePrompt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterPatch {
    pub enabled: Option<bool>,
    pub position: Option<CharacterPositionDto>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterPatch {
    pub size: Option<ImageSizeDto>,
    pub quality: Option<QualityPresetDto>,
    pub uc_preset: Option<UcPresetDto>,
    pub steps: Option<u32>,
    pub scale: Option<f32>,
    pub sampler: Option<SamplerDto>,
    pub noise_schedule: Option<NoiseScheduleDto>,
    pub seed_mode: Option<GenerationDraftSeedModeDto>,
    pub seed: Option<i64>,
    pub n_samples: Option<u32>,
    pub request_count: Option<u32>,
    pub cfg_rescale: Option<f32>,
    pub variety_boost: Option<bool>,
    pub strict_mode: Option<bool>,
    pub stream_enabled: Option<bool>,
    #[serde(default)]
    pub image_format: Patch<Option<ImageFormatDto>>,
    pub transparent_background: Option<bool>,
    pub furry_mode: Option<bool>,
    pub character_position_mode: Option<GenerationDraftCharacterPositionModeDto>,
}

impl DraftEdit {
    pub fn apply(&self, original: &GenerationDraftDto) -> AgentResult<GenerationDraftDto> {
        if self.operations.is_empty() {
            return Err(AgentError::validation("operations must not be empty"));
        }
        let mut draft = original.clone();
        for operation in &self.operations {
            operation.apply(&mut draft)?;
        }
        crate::mapping::generation_draft_to_domain(draft.clone())
            .validate()
            .map_err(|error| AgentError::validation(error.to_string()))?;
        Ok(draft)
    }
}

impl DraftOperation {
    fn apply(&self, draft: &mut GenerationDraftDto) -> AgentResult<()> {
        match self {
            Self::EditText { target, edit } => {
                let text = target.text_mut(draft)?;
                *text = edit.apply(text)?;
            }
            Self::CreateCharacter { id, prompt } => {
                insert_character(
                    draft,
                    GenerationDraftCharacterDto {
                        id: id.clone(),
                        preset_id: None,
                        prompt: prompt.clone(),
                        negative_prompt: String::new(),
                        enabled: true,
                        position: CharacterPositionDto { x: 0.5, y: 0.5 },
                    },
                )?;
            }
            Self::UpdateCharacter { id, patch } => {
                let character = character_mut(active_state(draft)?, id)?;
                if let Some(enabled) = patch.enabled {
                    character.enabled = enabled;
                }
                if let Some(position) = patch.position {
                    character.position = position;
                }
            }
            Self::CopyCharacter { id, new_id } => {
                let mut copy = character_mut(active_state(draft)?, id)?.clone();
                copy.id.clone_from(new_id);
                insert_character(draft, copy)?;
            }
            Self::RemoveCharacter { id } => {
                let state = active_state(draft)?;
                character_mut(state, id)?;
                state.characters.retain(|character| &character.id != id);
            }
            Self::ReorderCharacters { ids } => {
                let state = active_state(draft)?;
                let unique = ids.iter().collect::<std::collections::BTreeSet<_>>();
                if ids.len() != state.characters.len() || unique.len() != ids.len() {
                    return Err(AgentError::validation(
                        "ids must contain every character exactly once",
                    ));
                }
                state.characters = ids
                    .iter()
                    .map(|id| character_mut(state, id).cloned())
                    .collect::<AgentResult<_>>()?;
            }
            Self::SelectPreset {
                character_id,
                preset_id,
            } => {
                let state = active_state(draft)?;
                if let Some(id) = character_id {
                    character_mut(state, id)?.preset_id.clone_from(preset_id);
                } else {
                    state.main_preset_id.clone_from(preset_id);
                }
            }
            Self::SetParameters { patch } => patch.apply(draft)?,
            Self::SelectModel { model } => select_model(draft, *model),
        }
        Ok(())
    }
}

impl PromptTarget {
    fn text_mut<'a>(&self, draft: &'a mut GenerationDraftDto) -> AgentResult<&'a mut String> {
        let state = active_state(draft)?;
        if let Some(id) = &self.character_id {
            let character = character_mut(state, id)?;
            Ok(match self.field {
                PromptField::Prompt => &mut character.prompt,
                PromptField::NegativePrompt => &mut character.negative_prompt,
            })
        } else {
            Ok(match self.field {
                PromptField::Prompt => &mut state.prompt,
                PromptField::NegativePrompt => &mut state.negative_prompt,
            })
        }
    }
}

fn insert_character(
    draft: &mut GenerationDraftDto,
    character: GenerationDraftCharacterDto,
) -> AgentResult<()> {
    let max = crate::mapping::image_model_to_domain(draft.model)
        .capabilities()
        .max_characters;
    let state = active_state(draft)?;
    ensure_new_id(state, &character.id)?;
    if state.characters.len() >= max as usize {
        return Err(AgentError::validation("model character limit reached"));
    }
    state.characters.push(character);
    Ok(())
}

impl ParameterPatch {
    fn apply(&self, draft: &mut GenerationDraftDto) -> AgentResult<()> {
        macro_rules! assign {
            ($($field:ident),* $(,)?) => { $(if let Some(value) = self.$field { draft.$field = value; })* };
        }
        assign!(
            size,
            quality,
            uc_preset,
            steps,
            scale,
            sampler,
            noise_schedule,
            seed_mode,
            seed,
            n_samples,
            request_count,
            cfg_rescale,
            variety_boost,
            strict_mode,
            stream_enabled,
            transparent_background
        );
        self.image_format.apply(&mut draft.image_format);
        let state = active_state(draft)?;
        if let Some(value) = self.furry_mode {
            state.furry_mode = value;
        }
        if let Some(value) = self.character_position_mode {
            state.character_position_mode = value;
        }
        Ok(())
    }
}

fn active_state(draft: &mut GenerationDraftDto) -> AgentResult<&mut GenerationDraftPromptStateDto> {
    draft
        .prompt_states
        .iter_mut()
        .find(|state| state.model == draft.model)
        .ok_or_else(|| AgentError::validation("current model prompt state is missing"))
}

fn character_mut<'a>(
    state: &'a mut GenerationDraftPromptStateDto,
    id: &str,
) -> AgentResult<&'a mut GenerationDraftCharacterDto> {
    state
        .characters
        .iter_mut()
        .find(|character| character.id == id)
        .ok_or_else(|| AgentError::not_found("character does not exist"))
}

fn ensure_new_id(state: &GenerationDraftPromptStateDto, id: &str) -> AgentResult<()> {
    if id.trim().is_empty() || state.characters.iter().any(|character| character.id == id) {
        return Err(AgentError::validation(
            "character id must be nonempty and unique",
        ));
    }
    Ok(())
}

fn select_model(draft: &mut GenerationDraftDto, model: ImageModelDto) {
    if draft.model == model {
        return;
    }
    draft.model = model;
    draft.scale = crate::mapping::image_model_to_domain(model)
        .capabilities()
        .default_scale;
    if !draft.prompt_states.iter().any(|state| state.model == model) {
        draft.prompt_states.push(GenerationDraftPromptStateDto {
            model,
            main_preset_id: None,
            prompt: String::new(),
            negative_prompt: String::new(),
            furry_mode: false,
            characters: Vec::new(),
            character_position_mode: GenerationDraftCharacterPositionModeDto::Global,
        });
    }
}

#[cfg(test)]
mod tests;
