use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::{
    CharacterPromptTokenUsageDto, CountPromptTokensRequestDto, PromptTokenCountDto,
    PromptTokenUsageDto,
};
use atelier_generation::{
    Character, CharacterPosition, GenerateImageRequest, PromptTokenCount, count_prompt_tokens,
};
use atelier_prompt_resources::{
    CompileCharacterPromptRequest, CompileGenerationPromptRequest, PromptPresetId,
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use super::GenerationUseCases;
use super::generation_support::parse_uc_preset_override;
use crate::mapping::{image_model_to_domain, quality_preset_to_domain, uc_preset_to_domain};
use crate::{AppError, AppResult};

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn count_prompt_tokens(
        &self,
        request: CountPromptTokensRequestDto,
    ) -> AppResult<PromptTokenUsageDto> {
        let model = image_model_to_domain(request.compile.model);
        let character_inputs = request.compile.characters;
        let compiled = self
            .app
            .inner
            .prompt_compiler
            .compile_generation_prompt(CompileGenerationPromptRequest {
                model,
                main_preset_id: request.compile.main_preset_id.map(PromptPresetId::new),
                prompt: request.compile.prompt,
                negative_prompt: request.compile.negative_prompt.unwrap_or_default(),
                characters: character_inputs
                    .iter()
                    .enumerate()
                    .map(|(index, character)| CompileCharacterPromptRequest {
                        character_index: u32::try_from(index).unwrap_or(u32::MAX),
                        preset_id: character.preset_id.clone().map(PromptPresetId::new),
                        prompt: character.prompt.clone(),
                        negative_prompt: character.negative_prompt.clone().unwrap_or_default(),
                    })
                    .collect(),
                max_depth: request.compile.max_depth,
            })
            .await?;
        let quality = compiled
            .quality_override
            .unwrap_or_else(|| quality_preset_to_domain(request.quality));
        let uc_preset = match compiled.uc_preset_override.as_deref() {
            Some(value) => uc_preset_to_domain(parse_uc_preset_override(value)?),
            None => uc_preset_to_domain(request.uc_preset),
        };
        let characters = character_inputs
            .iter()
            .enumerate()
            .map(|(index, character)| {
                let compiled_character = compiled
                    .characters
                    .iter()
                    .find(|item| item.character_index == u32::try_from(index).unwrap_or(u32::MAX));
                Character {
                    prompt: compiled_character.map_or_else(String::new, |item| item.prompt.clone()),
                    negative_prompt: compiled_character.and_then(|item| {
                        (!item.negative_prompt.trim().is_empty())
                            .then(|| item.negative_prompt.clone())
                    }),
                    position: CharacterPosition::default(),
                    enabled: character.enabled,
                }
            })
            .collect::<Vec<_>>();
        let generation_request = GenerateImageRequest {
            prompt: compiled.prompt,
            model,
            negative_prompt: (!compiled.negative_prompt.trim().is_empty())
                .then_some(compiled.negative_prompt),
            quality,
            transparent_background: request.transparent_background,
            uc_preset,
            characters: (!characters.is_empty()).then_some(characters),
            ..GenerateImageRequest::default()
        };
        let usage = count_prompt_tokens(&generation_request)
            .map_err(|error| AppError::new("prompt_tokenizer", error.to_string()))?;
        Ok(PromptTokenUsageDto {
            prompt: prompt_token_count_to_dto(usage.prompt),
            negative_prompt: prompt_token_count_to_dto(usage.negative_prompt),
            characters: usage
                .characters
                .into_iter()
                .map(|character| CharacterPromptTokenUsageDto {
                    index: character.index,
                    prompt: prompt_token_count_to_dto(character.prompt),
                    negative_prompt: prompt_token_count_to_dto(character.negative_prompt),
                })
                .collect(),
        })
    }
}

const fn prompt_token_count_to_dto(value: PromptTokenCount) -> PromptTokenCountDto {
    PromptTokenCountDto {
        used: value.used,
        limit: value.limit,
    }
}
