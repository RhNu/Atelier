use crate::{GenerateImageRequest, QualityPreset, UcPreset};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PromptTokenCount {
    pub used: u32,
    pub limit: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterPromptTokenUsage {
    pub index: usize,
    pub prompt: PromptTokenCount,
    pub negative_prompt: PromptTokenCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTokenUsage {
    pub prompt: PromptTokenCount,
    pub negative_prompt: PromptTokenCount,
    pub characters: Vec<CharacterPromptTokenUsage>,
}

#[derive(Clone, Debug, thiserror::Error, PartialEq, Eq)]
#[error("prompt token counting failed: {message}")]
pub struct PromptTokenCountError {
    message: String,
}

/// Counts every effective prompt field after `NovelAI` request assembly.
///
/// The bridge owns request normalization, so these counts include model-input framing, quality
/// and transparency tags, the UC preset, and enabled character prompts exactly as transport
/// validation sees them. Atelier prompt functions must be compiled before calling this function.
///
/// # Errors
/// Returns an error when bundled tokenizer assets cannot be loaded or the text cannot be encoded.
pub fn count_prompt_tokens(
    request: &GenerateImageRequest,
) -> Result<PromptTokenUsage, PromptTokenCountError> {
    let tokenizers =
        novelai_bridge::Tokenizers::bundled().map_err(|error| PromptTokenCountError {
            message: error.to_string(),
        })?;
    let mut bridge_request = novelai_bridge::GenerateImageRequest::builder(
        request.prompt.clone(),
        request.model.bridge_model(),
    )
    .quality(quality_preset_to_bridge(request.quality))
    .furry_mode(request.furry_mode)
    .transparent_background(request.transparent_background)
    .uc_preset(uc_preset_to_bridge(request.uc_preset));
    if let Some(negative_prompt) = &request.negative_prompt {
        bridge_request = bridge_request.negative_prompt(negative_prompt.clone());
    }
    if let Some(characters) = &request.characters {
        bridge_request = bridge_request.characters(
            characters
                .iter()
                .map(|character| novelai_bridge::Character {
                    prompt: character.prompt.clone(),
                    negative_prompt: character.negative_prompt.clone(),
                    position: novelai_bridge::CharacterPosition {
                        x: character.position.x,
                        y: character.position.y,
                    },
                    enabled: character.enabled,
                })
                .collect(),
        );
    }
    let usage = bridge_request
        .build()
        .count_tokens(&tokenizers)
        .map_err(|error| PromptTokenCountError {
            message: error.to_string(),
        })?;
    Ok(PromptTokenUsage {
        prompt: prompt_token_count_from_bridge(usage.prompt),
        negative_prompt: prompt_token_count_from_bridge(usage.negative_prompt),
        characters: usage
            .characters
            .into_iter()
            .map(|character| CharacterPromptTokenUsage {
                index: character.index,
                prompt: prompt_token_count_from_bridge(character.prompt),
                negative_prompt: prompt_token_count_from_bridge(character.negative_prompt),
            })
            .collect(),
    })
}

const fn prompt_token_count_from_bridge(value: novelai_bridge::TokenCount) -> PromptTokenCount {
    PromptTokenCount {
        used: value.used,
        limit: value.limit,
    }
}

const fn quality_preset_to_bridge(value: QualityPreset) -> novelai_bridge::QualityPreset {
    match value {
        QualityPreset::Standard => novelai_bridge::QualityPreset::Standard,
        QualityPreset::Light => novelai_bridge::QualityPreset::Light,
        QualityPreset::None => novelai_bridge::QualityPreset::None,
    }
}

const fn uc_preset_to_bridge(value: UcPreset) -> novelai_bridge::UcPreset {
    match value {
        UcPreset::Heavy => novelai_bridge::UcPreset::Heavy,
        UcPreset::Light => novelai_bridge::UcPreset::Light,
        UcPreset::FurryFocus => novelai_bridge::UcPreset::FurryFocus,
        UcPreset::HumanFocus => novelai_bridge::UcPreset::HumanFocus,
        UcPreset::None => novelai_bridge::UcPreset::None,
    }
}
