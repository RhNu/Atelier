use crate::{
    Character, CharacterReference, GenerateImageRequest, GenerationError, ImageModel,
    Img2ImgRequest, QualityPreset, VibeTransferConfig,
};

const IMAGE_DIMENSION_MIN: u32 = 64;
const IMAGE_DIMENSION_MAX: u32 = 1600;
const IMAGE_DIMENSION_MULTIPLE: u32 = 64;
const GRID_CENTER: f32 = 0.5;
const GRID_TOLERANCE: f32 = 0.0001;

/// Validates and normalizes a `NovelAI` image generation request.
///
/// # Errors
/// Returns [`GenerationError`] when the prompt is empty, a strict request
/// contains out-of-range values, numeric values are non-finite, or requested
/// model features are incompatible.
pub fn normalize_generate_request(
    mut request: GenerateImageRequest,
) -> Result<GenerateImageRequest, GenerationError> {
    validate_prompt(&request)?;
    let strict_mode = request.strict_mode;
    normalize_base_fields(&mut request, strict_mode)?;
    normalize_model_features(&mut request)?;
    validate_character_inputs(request.characters.as_deref())?;
    normalize_character_positions(request.characters.as_mut(), strict_mode)?;
    normalize_character_references(request.character_references.as_mut(), strict_mode)?;
    normalize_i2i(request.img2img.as_mut(), strict_mode)?;
    normalize_vibe_transfer(request.vibe_transfer.as_mut(), strict_mode)?;
    Ok(request)
}

pub fn resolve_use_coords(request: &GenerateImageRequest) -> bool {
    let Some(characters) = request.characters.as_ref() else {
        return false;
    };
    let has_enabled = characters.iter().any(|character| character.enabled);
    if !has_enabled {
        return false;
    }
    if let Some(explicit) = request.use_coords {
        return explicit;
    }
    characters.iter().any(|character| {
        character.enabled
            && (!is_center_position(character.position.x)
                || !is_center_position(character.position.y))
    })
}

fn validate_prompt(request: &GenerateImageRequest) -> Result<(), GenerationError> {
    if request.prompt.trim().is_empty() {
        return Err(GenerationError::empty_field("prompt"));
    }
    Ok(())
}

fn normalize_base_fields(
    request: &mut GenerateImageRequest,
    strict_mode: bool,
) -> Result<(), GenerationError> {
    request.steps = normalize_u32_range("steps", request.steps, 1, 50, strict_mode)?;
    request.scale = normalize_f32_range("scale", request.scale, 0.0, 10.0, strict_mode)?;
    request.n_samples = normalize_u32_range("n_samples", request.n_samples, 1, 4, strict_mode)?;
    request.cfg_rescale =
        normalize_f32_range("cfg_rescale", request.cfg_rescale, 0.0, 1.0, strict_mode)?;
    request.size.width = normalize_image_dimension("size.width", request.size.width, strict_mode)?;
    request.size.height =
        normalize_image_dimension("size.height", request.size.height, strict_mode)?;
    Ok(())
}

fn normalize_model_features(request: &mut GenerateImageRequest) -> Result<(), GenerationError> {
    let descriptor = request.model.descriptor();
    normalize_character_reference_features(request)?;

    if request.model == ImageModel::NaiDiffusion5Curated
        && request
            .img2img
            .as_ref()
            .is_some_and(|i2i| i2i.inpaint.is_some())
    {
        return Err(GenerationError::unsupported_model_feature(
            "img2img.inpaint",
            "a model with dedicated inpainting support",
        ));
    }

    if request
        .characters
        .as_ref()
        .is_some_and(|characters| !characters.is_empty())
        && descriptor.prompt().max_characters() == 0
    {
        reject_or_clear(
            request.strict_mode,
            "characters",
            "models with character prompt support",
            || request.characters = None,
        )?;
    } else if request
        .characters
        .as_ref()
        .is_some_and(|characters| characters.len() > descriptor.prompt().max_characters() as usize)
    {
        if request.strict_mode {
            return Err(GenerationError::unsupported_model_feature(
                "characters",
                "the model character prompt limit",
            ));
        }
        request
            .characters
            .as_mut()
            .expect("checked above")
            .truncate(descriptor.prompt().max_characters() as usize);
    }

    if request
        .vibe_transfer
        .as_ref()
        .is_some_and(|vibe| !vibe.references.is_empty())
        && !descriptor.can_encode_vibe()
    {
        reject_or_clear(
            request.strict_mode,
            "vibe_transfer",
            "models with encoded vibe transfer support",
            || request.vibe_transfer = None,
        )?;
    }

    if request
        .vibe_transfer
        .as_ref()
        .is_some_and(|vibe| !vibe.references.is_empty())
        && request
            .character_references
            .as_ref()
            .is_some_and(|refs| !refs.is_empty())
    {
        return Err(GenerationError::unsupported_field_combination(
            "vibe_transfer+character_references",
            "vibe_transfer and character_references cannot be used together",
        ));
    }

    if request.variety_boost && descriptor.sampling().variety.is_none() {
        reject_or_clear(
            request.strict_mode,
            "variety_boost",
            "models with variety boost support",
            || request.variety_boost = false,
        )?;
    }
    if request.transparent_background && !descriptor.prompt().transparent_background {
        reject_or_clear(
            request.strict_mode,
            "transparent_background",
            "models with transparent background support",
            || request.transparent_background = false,
        )?;
    }
    if request.quality == QualityPreset::Light && !request.model.supports_light_quality_preset() {
        reject_or_clear(
            request.strict_mode,
            "quality.light",
            "models with the Light quality preset",
            || request.quality = QualityPreset::Standard,
        )?;
    }

    Ok(())
}

fn normalize_character_reference_features(
    request: &mut GenerateImageRequest,
) -> Result<(), GenerationError> {
    let has_references = request
        .character_references
        .as_ref()
        .is_some_and(|refs| !refs.is_empty());
    if !has_references {
        return Ok(());
    }

    let mode = match request.img2img.as_ref() {
        Some(i2i) if i2i.inpaint.is_some() => novelai_bridge::GenerationMode::Inpainting,
        Some(_) => novelai_bridge::GenerationMode::ImageToImage,
        None => novelai_bridge::GenerationMode::TextToImage,
    };
    let resolution = request.model.descriptor().resolve_references(
        mode,
        novelai_bridge::ReferenceIntent {
            vibe: false,
            character: true,
        },
    );

    if !resolution.character.is_available() {
        let field = if matches!(mode, novelai_bridge::GenerationMode::Inpainting) {
            "character_references with img2img.inpaint"
        } else {
            "character_references"
        };
        let supported_by = if matches!(mode, novelai_bridge::GenerationMode::Inpainting) {
            "models with precise reference inpainting support"
        } else {
            "models with precise reference support"
        };
        reject_or_clear(request.strict_mode, field, supported_by, || {
            request.character_references = None;
        })?;
    }

    Ok(())
}

fn reject_or_clear(
    strict_mode: bool,
    field: &'static str,
    supported_by: &'static str,
    clear: impl FnOnce(),
) -> Result<(), GenerationError> {
    if strict_mode {
        return Err(GenerationError::unsupported_model_feature(
            field,
            supported_by,
        ));
    }
    clear();
    Ok(())
}

fn validate_character_inputs(characters: Option<&[Character]>) -> Result<(), GenerationError> {
    let Some(characters) = characters else {
        return Ok(());
    };

    for (idx, character) in characters.iter().enumerate() {
        if character.prompt.trim().is_empty() {
            return Err(GenerationError::empty_field(format!(
                "characters[{idx}].prompt"
            )));
        }
    }

    Ok(())
}

fn normalize_character_positions(
    characters: Option<&mut Vec<Character>>,
    strict_mode: bool,
) -> Result<(), GenerationError> {
    let Some(characters) = characters else {
        return Ok(());
    };

    for (idx, character) in characters.iter_mut().enumerate() {
        character.position.x = normalize_f32_range(
            &format!("characters[{idx}].position.x"),
            character.position.x,
            0.0,
            1.0,
            strict_mode,
        )?;
        character.position.y = normalize_f32_range(
            &format!("characters[{idx}].position.y"),
            character.position.y,
            0.0,
            1.0,
            strict_mode,
        )?;
    }

    Ok(())
}

fn normalize_character_references(
    references: Option<&mut Vec<CharacterReference>>,
    strict_mode: bool,
) -> Result<(), GenerationError> {
    let Some(references) = references else {
        return Ok(());
    };

    for (idx, reference) in references.iter_mut().enumerate() {
        reference.fidelity = normalize_f32_range(
            &format!("character_references[{idx}].fidelity"),
            reference.fidelity,
            0.0,
            1.0,
            strict_mode,
        )?;
        reference.strength = normalize_f32_range(
            &format!("character_references[{idx}].strength"),
            reference.strength,
            0.0,
            1.0,
            strict_mode,
        )?;
    }

    Ok(())
}

fn normalize_i2i(
    i2i: Option<&mut Img2ImgRequest>,
    strict_mode: bool,
) -> Result<(), GenerationError> {
    let Some(i2i) = i2i else {
        return Ok(());
    };

    i2i.strength = normalize_f32_range(
        "i2i.strength",
        i2i.strength,
        0.01,
        if i2i.inpaint.is_some() { 1.0 } else { 0.99 },
        strict_mode,
    )?;
    i2i.noise = normalize_f32_range("i2i.noise", i2i.noise, 0.0, 0.99, strict_mode)?;
    Ok(())
}

fn normalize_vibe_transfer(
    vibe_transfer: Option<&mut VibeTransferConfig>,
    strict_mode: bool,
) -> Result<(), GenerationError> {
    let Some(vibe_transfer) = vibe_transfer else {
        return Ok(());
    };

    vibe_transfer.strength = normalize_f32_range(
        "vibe_transfer.strength",
        vibe_transfer.strength,
        0.0,
        1.0,
        strict_mode,
    )?;
    if vibe_transfer.references.is_empty() {
        return Err(GenerationError::empty_field("vibe_transfer.references"));
    }

    for (idx, reference) in vibe_transfer.references.iter_mut().enumerate() {
        if reference.vibe_data_cache.trim().is_empty() {
            return Err(GenerationError::empty_field(format!(
                "vibe_transfer.references[{idx}].vibe_data_cache"
            )));
        }
        reference.strength = normalize_f32_range(
            &format!("vibe_transfer.references[{idx}].strength"),
            reference.strength,
            0.0,
            1.0,
            strict_mode,
        )?;
    }

    Ok(())
}

fn normalize_u32_range(
    field: &str,
    value: u32,
    min: u32,
    max: u32,
    strict_mode: bool,
) -> Result<u32, GenerationError> {
    if strict_mode && !(min..=max).contains(&value) {
        return Err(GenerationError::numeric_out_of_range(field, min, max));
    }
    Ok(value.clamp(min, max))
}

fn normalize_f32_range(
    field: &str,
    value: f32,
    min: f32,
    max: f32,
    strict_mode: bool,
) -> Result<f32, GenerationError> {
    if !value.is_finite() {
        return Err(GenerationError::non_finite_number(field));
    }
    if strict_mode && !(min..=max).contains(&value) {
        return Err(GenerationError::numeric_out_of_range(field, min, max));
    }
    Ok(value.clamp(min, max))
}

fn normalize_image_dimension(
    field: &str,
    value: u32,
    strict_mode: bool,
) -> Result<u32, GenerationError> {
    if strict_mode
        && (!(IMAGE_DIMENSION_MIN..=IMAGE_DIMENSION_MAX).contains(&value)
            || !value.is_multiple_of(IMAGE_DIMENSION_MULTIPLE))
    {
        return Err(GenerationError::invalid_image_dimension(field));
    }

    let clamped = value.clamp(IMAGE_DIMENSION_MIN, IMAGE_DIMENSION_MAX);
    let snapped = ((clamped + (IMAGE_DIMENSION_MULTIPLE / 2)) / IMAGE_DIMENSION_MULTIPLE)
        * IMAGE_DIMENSION_MULTIPLE;
    Ok(snapped.clamp(IMAGE_DIMENSION_MIN, IMAGE_DIMENSION_MAX))
}

fn is_center_position(value: f32) -> bool {
    (value - GRID_CENTER).abs() <= GRID_TOLERANCE
}
