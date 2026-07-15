use atelier_app_api::generation::{
    CharacterPositionDto, GenerationDraftCharacterDto, GenerationDraftCharacterPositionModeDto,
    GenerationDraftDto, GenerationDraftI2iDto, GenerationDraftPreciseReferenceDto,
    GenerationDraftSeedModeDto, GenerationDraftVibeDto, GenerationDraftVibeSlotDto,
};
use atelier_generation::{
    CharacterPosition, CharacterReferenceType, GenerationDraftCharacter,
    GenerationDraftCharacterPositionMode, GenerationDraftI2i, GenerationDraftPreciseReference,
    GenerationDraftSeedMode, GenerationDraftSnapshot, GenerationDraftVibe, GenerationDraftVibeSlot,
};

use super::{
    image_format_to_domain, image_format_to_dto, image_model_to_domain, image_model_to_dto,
    noise_schedule_to_domain, noise_schedule_to_dto, resource_ref_from_dto, resource_ref_to_dto,
    sampler_to_domain, sampler_to_dto, uc_preset_to_domain, uc_preset_to_dto,
};

pub fn generation_draft_to_domain(value: GenerationDraftDto) -> GenerationDraftSnapshot {
    GenerationDraftSnapshot {
        main_preset_id: value.main_preset_id,
        prompt: value.prompt,
        negative_prompt: value.negative_prompt,
        model: image_model_to_domain(value.model),
        size: atelier_generation::ImageSize {
            width: value.size.width,
            height: value.size.height,
        },
        quality: value.quality,
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
        characters: value
            .characters
            .into_iter()
            .map(character_to_domain)
            .collect(),
        character_position_mode: position_mode_to_domain(value.character_position_mode),
    }
}

pub fn generation_draft_to_dto(value: &GenerationDraftSnapshot) -> GenerationDraftDto {
    GenerationDraftDto {
        main_preset_id: value.main_preset_id.clone(),
        prompt: value.prompt.clone(),
        negative_prompt: value.negative_prompt.clone(),
        model: image_model_to_dto(value.model),
        size: atelier_app_api::generation::ImageSizeDto {
            width: value.size.width,
            height: value.size.height,
        },
        quality: value.quality,
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
        characters: value.characters.iter().map(character_to_dto).collect(),
        character_position_mode: position_mode_to_dto(value.character_position_mode),
    }
}

fn i2i_to_domain(value: GenerationDraftI2iDto) -> GenerationDraftI2i {
    GenerationDraftI2i {
        image: resource_ref_from_dto(value.image),
        mask: value.mask.map(resource_ref_from_dto),
        strength: value.strength,
        noise: value.noise,
    }
}

fn i2i_to_dto(value: &GenerationDraftI2i) -> GenerationDraftI2iDto {
    GenerationDraftI2iDto {
        image: resource_ref_to_dto(&value.image),
        mask: value.mask.as_ref().map(resource_ref_to_dto),
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
            })
            .collect(),
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
    }
}
