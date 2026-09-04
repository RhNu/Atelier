use atelier_app_api::generation::{
    CharacterPositionDto, GenerationDraftCharacterDto, GenerationDraftCharacterPositionModeDto,
    GenerationDraftDto, GenerationDraftFocusRegionDto, GenerationDraftI2iDto,
    GenerationDraftInpaintSessionDto, GenerationDraftMaskDisplayDto, GenerationDraftMaskPatternDto,
    GenerationDraftPreciseReferenceDto, GenerationDraftPromptStateDto,
    GenerationDraftReferenceInsetDto, GenerationDraftSeedModeDto, GenerationDraftVibeDto,
    GenerationDraftVibeSlotDto,
};
use atelier_generation::{
    CharacterPosition, CharacterReferenceType, GenerationDraftCharacter,
    GenerationDraftCharacterPositionMode, GenerationDraftFocusRegion, GenerationDraftI2i,
    GenerationDraftInpaintSession, GenerationDraftMaskDisplay, GenerationDraftMaskPattern,
    GenerationDraftPreciseReference, GenerationDraftPromptState, GenerationDraftReferenceInset,
    GenerationDraftSeedMode, GenerationDraftSnapshot, GenerationDraftVibe, GenerationDraftVibeSlot,
};

use super::{
    image_format_to_domain, image_format_to_dto, image_model_to_domain, image_model_to_dto,
    noise_schedule_to_domain, noise_schedule_to_dto, quality_preset_to_domain,
    quality_preset_to_dto, resource_ref_from_dto, resource_ref_to_dto, sampler_to_domain,
    sampler_to_dto, uc_preset_to_domain, uc_preset_to_dto,
};

pub fn generation_draft_to_domain(value: GenerationDraftDto) -> GenerationDraftSnapshot {
    GenerationDraftSnapshot {
        model: image_model_to_domain(value.model),
        prompt_states: value
            .prompt_states
            .into_iter()
            .map(prompt_state_to_domain)
            .collect(),
        size: atelier_generation::ImageSize {
            width: value.size.width,
            height: value.size.height,
        },
        quality: quality_preset_to_domain(value.quality),
        transparent_background: value.transparent_background,
        uc_preset: uc_preset_to_domain(value.uc_preset),
        steps: value.steps,
        scale: value.scale,
        sampler: sampler_to_domain(value.sampler),
        noise_schedule: noise_schedule_to_domain(value.noise_schedule),
        seed_mode: seed_mode_to_domain(value.seed_mode),
        seed: value.seed,
        n_samples: value.n_samples,
        request_count: value.request_count,
        cfg_rescale: value.cfg_rescale,
        variety_boost: value.variety_boost,
        image_format: value.image_format.map(image_format_to_domain),
        strict_mode: value.strict_mode,
        stream_enabled: value.stream_enabled,
        i2i: value.i2i.map(i2i_to_domain),
        vibe: vibe_to_domain(value.vibe),
        precise_references: value
            .precise_references
            .into_iter()
            .map(precise_reference_to_domain)
            .collect(),
    }
}

pub fn generation_draft_to_dto(value: &GenerationDraftSnapshot) -> GenerationDraftDto {
    GenerationDraftDto {
        model: image_model_to_dto(value.model),
        prompt_states: value
            .prompt_states
            .iter()
            .map(prompt_state_to_dto)
            .collect(),
        size: atelier_app_api::generation::ImageSizeDto {
            width: value.size.width,
            height: value.size.height,
        },
        quality: quality_preset_to_dto(value.quality),
        transparent_background: value.transparent_background,
        uc_preset: uc_preset_to_dto(value.uc_preset),
        steps: value.steps,
        scale: value.scale,
        sampler: sampler_to_dto(value.sampler),
        noise_schedule: noise_schedule_to_dto(value.noise_schedule),
        seed_mode: seed_mode_to_dto(value.seed_mode),
        seed: value.seed,
        n_samples: value.n_samples,
        request_count: value.request_count,
        cfg_rescale: value.cfg_rescale,
        variety_boost: value.variety_boost,
        image_format: value.image_format.map(image_format_to_dto),
        strict_mode: value.strict_mode,
        stream_enabled: value.stream_enabled,
        i2i: value.i2i.as_ref().map(i2i_to_dto),
        vibe: vibe_to_dto(&value.vibe),
        precise_references: value
            .precise_references
            .iter()
            .map(precise_reference_to_dto)
            .collect(),
    }
}

fn i2i_to_domain(value: GenerationDraftI2iDto) -> GenerationDraftI2i {
    GenerationDraftI2i {
        image: resource_ref_from_dto(value.image),
        inpaint: value.inpaint.map(|inpaint| GenerationDraftInpaintSession {
            region_to_replace: resource_ref_from_dto(inpaint.region_to_replace),
            display: GenerationDraftMaskDisplay {
                color: inpaint.display.color,
                opacity: inpaint.display.opacity,
                pattern: match inpaint.display.pattern {
                    GenerationDraftMaskPatternDto::Solid => GenerationDraftMaskPattern::Solid,
                    GenerationDraftMaskPatternDto::Stripes => GenerationDraftMaskPattern::Stripes,
                },
                show_border: inpaint.display.show_border,
                brush_size: inpaint.display.brush_size,
            },
            focus: inpaint.focus.map(|focus| GenerationDraftFocusRegion {
                x: focus.x,
                y: focus.y,
                width: focus.width,
                height: focus.height,
                minimum_context_area: focus.minimum_context_area,
            }),
            reference_insets: inpaint
                .reference_insets
                .into_iter()
                .map(|inset| GenerationDraftReferenceInset {
                    id: inset.id,
                    image: resource_ref_from_dto(inset.image),
                    x: inset.x,
                    y: inset.y,
                    width: inset.width,
                    height: inset.height,
                    border_enabled: inset.border_enabled,
                    border_width: inset.border_width,
                })
                .collect(),
        }),
        strength: value.strength,
        noise: value.noise,
    }
}

fn i2i_to_dto(value: &GenerationDraftI2i) -> GenerationDraftI2iDto {
    GenerationDraftI2iDto {
        image: resource_ref_to_dto(&value.image),
        inpaint: value
            .inpaint
            .as_ref()
            .map(|inpaint| GenerationDraftInpaintSessionDto {
                region_to_replace: resource_ref_to_dto(&inpaint.region_to_replace),
                display: GenerationDraftMaskDisplayDto {
                    color: inpaint.display.color.clone(),
                    opacity: inpaint.display.opacity,
                    pattern: match inpaint.display.pattern {
                        GenerationDraftMaskPattern::Solid => GenerationDraftMaskPatternDto::Solid,
                        GenerationDraftMaskPattern::Stripes => {
                            GenerationDraftMaskPatternDto::Stripes
                        }
                    },
                    show_border: inpaint.display.show_border,
                    brush_size: inpaint.display.brush_size,
                },
                focus: inpaint.focus.map(|focus| GenerationDraftFocusRegionDto {
                    x: focus.x,
                    y: focus.y,
                    width: focus.width,
                    height: focus.height,
                    minimum_context_area: focus.minimum_context_area,
                }),
                reference_insets: inpaint
                    .reference_insets
                    .iter()
                    .map(|inset| GenerationDraftReferenceInsetDto {
                        id: inset.id.clone(),
                        image: resource_ref_to_dto(&inset.image),
                        x: inset.x,
                        y: inset.y,
                        width: inset.width,
                        height: inset.height,
                        border_enabled: inset.border_enabled,
                        border_width: inset.border_width,
                    })
                    .collect(),
            }),
        strength: value.strength,
        noise: value.noise,
    }
}

fn vibe_to_domain(value: GenerationDraftVibeDto) -> GenerationDraftVibe {
    GenerationDraftVibe {
        enabled: value.enabled,
        strength: value.strength,
        slots: value
            .slots
            .into_iter()
            .map(|slot| GenerationDraftVibeSlot {
                id: slot.id,
                encoding: resource_ref_from_dto(slot.encoding),
                vibe_id: slot.vibe_id,
                information_extracted: slot.information_extracted,
                strength: slot.strength,
                display_name: slot.display_name,
                source_image: slot.source_image.map(resource_ref_from_dto),
                source_sha256: slot.source_sha256,
                model: image_model_to_domain(slot.model),
            })
            .collect(),
    }
}

fn vibe_to_dto(value: &GenerationDraftVibe) -> GenerationDraftVibeDto {
    GenerationDraftVibeDto {
        enabled: value.enabled,
        strength: value.strength,
        slots: value
            .slots
            .iter()
            .map(|slot| GenerationDraftVibeSlotDto {
                id: slot.id.clone(),
                encoding: resource_ref_to_dto(&slot.encoding),
                vibe_id: slot.vibe_id.clone(),
                information_extracted: slot.information_extracted,
                strength: slot.strength,
                display_name: slot.display_name.clone(),
                source_image: slot.source_image.as_ref().map(resource_ref_to_dto),
                source_sha256: slot.source_sha256.clone(),
                model: image_model_to_dto(slot.model),
            })
            .collect(),
    }
}

fn prompt_state_to_domain(value: GenerationDraftPromptStateDto) -> GenerationDraftPromptState {
    GenerationDraftPromptState {
        model: image_model_to_domain(value.model),
        main_preset_id: value.main_preset_id,
        prompt: value.prompt,
        negative_prompt: value.negative_prompt,
        furry_mode: value.furry_mode,
        characters: value
            .characters
            .into_iter()
            .map(character_to_domain)
            .collect(),
        character_position_mode: position_mode_to_domain(value.character_position_mode),
    }
}

fn prompt_state_to_dto(value: &GenerationDraftPromptState) -> GenerationDraftPromptStateDto {
    GenerationDraftPromptStateDto {
        model: image_model_to_dto(value.model),
        main_preset_id: value.main_preset_id.clone(),
        prompt: value.prompt.clone(),
        negative_prompt: value.negative_prompt.clone(),
        furry_mode: value.furry_mode,
        characters: value.characters.iter().map(character_to_dto).collect(),
        character_position_mode: position_mode_to_dto(value.character_position_mode),
    }
}

fn precise_reference_to_domain(
    value: GenerationDraftPreciseReferenceDto,
) -> GenerationDraftPreciseReference {
    GenerationDraftPreciseReference {
        id: value.id,
        image: resource_ref_from_dto(value.image),
        reference_type: character_reference_type_to_domain(value.reference_type),
        fidelity: value.fidelity,
        strength: value.strength,
        display_name: value.display_name,
    }
}

fn precise_reference_to_dto(
    value: &GenerationDraftPreciseReference,
) -> GenerationDraftPreciseReferenceDto {
    GenerationDraftPreciseReferenceDto {
        id: value.id.clone(),
        image: resource_ref_to_dto(&value.image),
        reference_type: character_reference_type_to_dto(value.reference_type),
        fidelity: value.fidelity,
        strength: value.strength,
        display_name: value.display_name.clone(),
    }
}

fn character_to_domain(value: GenerationDraftCharacterDto) -> GenerationDraftCharacter {
    GenerationDraftCharacter {
        id: value.id,
        preset_id: value.preset_id,
        prompt: value.prompt,
        negative_prompt: value.negative_prompt,
        enabled: value.enabled,
        position: CharacterPosition {
            x: value.position.x,
            y: value.position.y,
        },
    }
}

fn character_to_dto(value: &GenerationDraftCharacter) -> GenerationDraftCharacterDto {
    GenerationDraftCharacterDto {
        id: value.id.clone(),
        preset_id: value.preset_id.clone(),
        prompt: value.prompt.clone(),
        negative_prompt: value.negative_prompt.clone(),
        enabled: value.enabled,
        position: CharacterPositionDto {
            x: value.position.x,
            y: value.position.y,
        },
    }
}

const fn seed_mode_to_domain(value: GenerationDraftSeedModeDto) -> GenerationDraftSeedMode {
    match value {
        GenerationDraftSeedModeDto::Random => GenerationDraftSeedMode::Random,
        GenerationDraftSeedModeDto::Fixed => GenerationDraftSeedMode::Fixed,
    }
}

const fn seed_mode_to_dto(value: GenerationDraftSeedMode) -> GenerationDraftSeedModeDto {
    match value {
        GenerationDraftSeedMode::Random => GenerationDraftSeedModeDto::Random,
        GenerationDraftSeedMode::Fixed => GenerationDraftSeedModeDto::Fixed,
    }
}

const fn position_mode_to_domain(
    value: GenerationDraftCharacterPositionModeDto,
) -> GenerationDraftCharacterPositionMode {
    match value {
        GenerationDraftCharacterPositionModeDto::Global => {
            GenerationDraftCharacterPositionMode::Global
        }
        GenerationDraftCharacterPositionModeDto::Manual => {
            GenerationDraftCharacterPositionMode::Manual
        }
    }
}

const fn position_mode_to_dto(
    value: GenerationDraftCharacterPositionMode,
) -> GenerationDraftCharacterPositionModeDto {
    match value {
        GenerationDraftCharacterPositionMode::Global => {
            GenerationDraftCharacterPositionModeDto::Global
        }
        GenerationDraftCharacterPositionMode::Manual => {
            GenerationDraftCharacterPositionModeDto::Manual
        }
    }
}

const fn character_reference_type_to_dto(
    value: CharacterReferenceType,
) -> atelier_app_api::generation::CharacterReferenceTypeDto {
    match value {
        CharacterReferenceType::Character => {
            atelier_app_api::generation::CharacterReferenceTypeDto::Character
        }
        CharacterReferenceType::Style => {
            atelier_app_api::generation::CharacterReferenceTypeDto::Style
        }
        CharacterReferenceType::CharacterAndStyle => {
            atelier_app_api::generation::CharacterReferenceTypeDto::CharacterAndStyle
        }
        CharacterReferenceType::Costume => {
            atelier_app_api::generation::CharacterReferenceTypeDto::Costume
        }
        CharacterReferenceType::Delta => {
            atelier_app_api::generation::CharacterReferenceTypeDto::Delta
        }
    }
}

const fn character_reference_type_to_domain(
    value: atelier_app_api::generation::CharacterReferenceTypeDto,
) -> CharacterReferenceType {
    match value {
        atelier_app_api::generation::CharacterReferenceTypeDto::Character => {
            CharacterReferenceType::Character
        }
        atelier_app_api::generation::CharacterReferenceTypeDto::Style => {
            CharacterReferenceType::Style
        }
        atelier_app_api::generation::CharacterReferenceTypeDto::CharacterAndStyle => {
            CharacterReferenceType::CharacterAndStyle
        }
        atelier_app_api::generation::CharacterReferenceTypeDto::Costume => {
            CharacterReferenceType::Costume
        }
        atelier_app_api::generation::CharacterReferenceTypeDto::Delta => {
            CharacterReferenceType::Delta
        }
    }
}
