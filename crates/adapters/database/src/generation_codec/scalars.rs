use super::{
    CharacterReferenceType, DatabaseResult, ImageFormat, ImageModel, NoiseSchedule, Sampler,
    StreamMode, UcPreset, decode_error,
};

pub const fn image_model_as_str(value: ImageModel) -> &'static str {
    value.as_str()
}

pub fn image_model_from_str(value: &str) -> DatabaseResult<ImageModel> {
    match value {
        "nai-diffusion-4-5-full" => Ok(ImageModel::NaiDiffusion45Full),
        "nai-diffusion-4-5-curated" => Ok(ImageModel::NaiDiffusion45Curated),
        "nai-diffusion-4-full" => Ok(ImageModel::NaiDiffusion4Full),
        "nai-diffusion-4-curated" => Ok(ImageModel::NaiDiffusion4Curated),
        "nai-diffusion-3" => Ok(ImageModel::NaiDiffusion3),
        "nai-diffusion-3-furry" => Ok(ImageModel::NaiDiffusion3Furry),
        _ => Err(decode_error("image model", value)),
    }
}

pub const fn sampler_as_str(value: Sampler) -> &'static str {
    match value {
        Sampler::KEuler => "k_euler",
        Sampler::KEulerAncestral => "k_euler_ancestral",
        Sampler::KDpm2 => "k_dpm2",
        Sampler::KDpm2Ancestral => "k_dpm2_ancestral",
        Sampler::KDpmpp2m => "k_dpmpp_2m",
        Sampler::KDpmpp2sAncestral => "k_dpmpp_2s_ancestral",
        Sampler::KDpmppSde => "k_dpmpp_sde",
        Sampler::Ddim => "ddim",
    }
}

pub fn sampler_from_str(value: &str) -> DatabaseResult<Sampler> {
    match value {
        "k_euler" => Ok(Sampler::KEuler),
        "k_euler_ancestral" => Ok(Sampler::KEulerAncestral),
        "k_dpm2" => Ok(Sampler::KDpm2),
        "k_dpm2_ancestral" => Ok(Sampler::KDpm2Ancestral),
        "k_dpmpp_2m" => Ok(Sampler::KDpmpp2m),
        "k_dpmpp_2s_ancestral" => Ok(Sampler::KDpmpp2sAncestral),
        "k_dpmpp_sde" => Ok(Sampler::KDpmppSde),
        "ddim" => Ok(Sampler::Ddim),
        _ => Err(decode_error("sampler", value)),
    }
}

pub const fn noise_schedule_as_str(value: NoiseSchedule) -> &'static str {
    match value {
        NoiseSchedule::Karras => "karras",
        NoiseSchedule::Exponential => "exponential",
        NoiseSchedule::Polyexponential => "polyexponential",
    }
}

pub fn noise_schedule_from_str(value: &str) -> DatabaseResult<NoiseSchedule> {
    match value {
        "karras" => Ok(NoiseSchedule::Karras),
        "exponential" => Ok(NoiseSchedule::Exponential),
        "polyexponential" => Ok(NoiseSchedule::Polyexponential),
        _ => Err(decode_error("noise schedule", value)),
    }
}

pub const fn uc_preset_as_str(value: UcPreset) -> &'static str {
    match value {
        UcPreset::Heavy => "heavy",
        UcPreset::Light => "light",
        UcPreset::FurryFocus => "furry_focus",
        UcPreset::HumanFocus => "human_focus",
        UcPreset::None => "none",
    }
}

pub fn uc_preset_from_str(value: &str) -> DatabaseResult<UcPreset> {
    match value {
        "heavy" => Ok(UcPreset::Heavy),
        "light" => Ok(UcPreset::Light),
        "furry_focus" => Ok(UcPreset::FurryFocus),
        "human_focus" => Ok(UcPreset::HumanFocus),
        "none" => Ok(UcPreset::None),
        _ => Err(decode_error("uc preset", value)),
    }
}

pub const fn image_format_as_str(value: ImageFormat) -> &'static str {
    match value {
        ImageFormat::Png => "png",
        ImageFormat::Webp => "webp",
    }
}

pub fn image_format_from_str(value: &str) -> DatabaseResult<ImageFormat> {
    match value {
        "png" => Ok(ImageFormat::Png),
        "webp" => Ok(ImageFormat::Webp),
        _ => Err(decode_error("image format", value)),
    }
}

pub const fn stream_mode_as_str(value: StreamMode) -> &'static str {
    match value {
        StreamMode::Sse => "sse",
    }
}

pub fn stream_mode_from_str(value: &str) -> DatabaseResult<StreamMode> {
    match value {
        "sse" => Ok(StreamMode::Sse),
        _ => Err(decode_error("stream mode", value)),
    }
}

pub const fn character_reference_type_as_str(value: CharacterReferenceType) -> &'static str {
    match value {
        CharacterReferenceType::Character => "character",
        CharacterReferenceType::Style => "style",
        CharacterReferenceType::CharacterAndStyle => "character_and_style",
    }
}

pub fn character_reference_type_from_str(value: &str) -> DatabaseResult<CharacterReferenceType> {
    match value {
        "character" => Ok(CharacterReferenceType::Character),
        "style" => Ok(CharacterReferenceType::Style),
        "character_and_style" => Ok(CharacterReferenceType::CharacterAndStyle),
        _ => Err(decode_error("character reference type", value)),
    }
}
