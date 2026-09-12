use crate::mapping::image_model_to_domain;
use atelier_app_api::{
    generation::GenerateImageRequestDto,
    prompt::{CompileGenerationCharacterPromptDto, CompileGenerationPromptRequestDto},
};
use atelier_prompt_resources::{
    CompileCharacterPromptRequest, CompileGenerationPromptRequest, PromptPresetId,
};

pub fn compile_request(
    request: CompileGenerationPromptRequestDto,
) -> CompileGenerationPromptRequest {
    CompileGenerationPromptRequest {
        model: image_model_to_domain(request.model),
        main_preset_id: request.main_preset_id.map(PromptPresetId::new),
        prompt: request.prompt,
        negative_prompt: request.negative_prompt.unwrap_or_default(),
        characters: request
            .characters
            .into_iter()
            .enumerate()
            .map(|(index, character)| CompileCharacterPromptRequest {
                character_index: u32::try_from(index).unwrap_or(u32::MAX),
                preset_id: character.preset_id.map(PromptPresetId::new),
                prompt: character.prompt,
                negative_prompt: character.negative_prompt.unwrap_or_default(),
            })
            .collect(),
        max_depth: request.max_depth,
    }
}

pub fn generation_prompt(request: &GenerateImageRequestDto) -> CompileGenerationPromptRequestDto {
    CompileGenerationPromptRequestDto {
        model: request.model,
        main_preset_id: request.main_preset_id.clone(),
        prompt: request.prompt.clone(),
        negative_prompt: request.negative_prompt.clone(),
        characters: request
            .characters
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|character| CompileGenerationCharacterPromptDto {
                preset_id: character.preset_id.clone(),
                prompt: character.prompt.clone(),
                negative_prompt: character.negative_prompt.clone(),
                enabled: character.enabled,
            })
            .collect(),
        max_depth: 16,
    }
}
