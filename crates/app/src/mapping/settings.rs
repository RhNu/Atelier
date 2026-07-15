use super::{
    AppResult, GenerationDefaults, GenerationDefaultsDto, GlobalFrontendSettings,
    GlobalFrontendSettingsDto, GlobalGallerySettings, GlobalGallerySettingsDto, GlobalSettings,
    GlobalSettingsDto, ImageSize, ImageSizeDto, ImageVariantSettings, ImageVariantSettingsDto,
    WorkspaceSettings, WorkspaceSettingsDto, image_format_to_domain, image_format_to_dto,
    image_model_to_domain, image_model_to_dto, noise_schedule_to_domain, noise_schedule_to_dto,
    sampler_to_domain, sampler_to_dto, uc_preset_to_domain, uc_preset_to_dto,
};

pub fn workspace_settings_to_dto(value: &WorkspaceSettings) -> WorkspaceSettingsDto {
    WorkspaceSettingsDto {
        generation: generation_defaults_to_dto(&value.generation),
        image_variants: image_variant_settings_to_dto(value.image_variants),
    }
}

pub fn workspace_settings_to_domain(value: &WorkspaceSettingsDto) -> AppResult<WorkspaceSettings> {
    let settings = WorkspaceSettings {
        generation: generation_defaults_to_domain(&value.generation),
        image_variants: image_variant_settings_to_domain(value.image_variants),
    };
    settings.validate()?;
    Ok(settings)
}

pub fn global_settings_to_dto(value: &GlobalSettings) -> GlobalSettingsDto {
    GlobalSettingsDto {
        last_workspace: value.last_workspace.clone(),
        frontend: global_frontend_settings_to_dto(value.frontend),
    }
}

pub const fn global_frontend_settings_to_domain(
    value: GlobalFrontendSettingsDto,
) -> GlobalFrontendSettings {
    GlobalFrontendSettings {
        developer_mode: value.developer_mode,
        gallery: GlobalGallerySettings {
            blur_sensitive_images: value.gallery.blur_sensitive_images,
        },
    }
}

const fn global_frontend_settings_to_dto(
    value: GlobalFrontendSettings,
) -> GlobalFrontendSettingsDto {
    GlobalFrontendSettingsDto {
        developer_mode: value.developer_mode,
        gallery: GlobalGallerySettingsDto {
            blur_sensitive_images: value.gallery.blur_sensitive_images,
        },
    }
}

fn generation_defaults_to_dto(value: &GenerationDefaults) -> GenerationDefaultsDto {
    GenerationDefaultsDto {
        model: image_model_to_dto(value.model),
        size: ImageSizeDto {
            width: value.size.width,
            height: value.size.height,
        },
        quality: value.quality,
        uc_preset: uc_preset_to_dto(value.uc_preset),
        steps: value.steps,
        scale: value.scale,
        sampler: sampler_to_dto(value.sampler),
        noise_schedule: noise_schedule_to_dto(value.noise_schedule),
        seed: value.seed,
        n_samples: value.n_samples,
        cfg_rescale: value.cfg_rescale,
        variety_boost: value.variety_boost,
        image_format: value.image_format.map(image_format_to_dto),
        strict_mode: value.strict_mode,
    }
}

fn generation_defaults_to_domain(value: &GenerationDefaultsDto) -> GenerationDefaults {
    GenerationDefaults {
        model: image_model_to_domain(value.model),
        size: ImageSize {
            width: value.size.width,
            height: value.size.height,
        },
        quality: value.quality,
        uc_preset: uc_preset_to_domain(value.uc_preset),
        steps: value.steps,
        scale: value.scale,
        sampler: sampler_to_domain(value.sampler),
        noise_schedule: noise_schedule_to_domain(value.noise_schedule),
        seed: value.seed,
        n_samples: value.n_samples,
        cfg_rescale: value.cfg_rescale,
        variety_boost: value.variety_boost,
        image_format: value.image_format.map(image_format_to_domain),
        strict_mode: value.strict_mode,
    }
}

const fn image_variant_settings_to_dto(value: ImageVariantSettings) -> ImageVariantSettingsDto {
    ImageVariantSettingsDto {
        thumbnail_long_edge: value.thumbnail_long_edge,
        preview_long_edge: value.preview_long_edge,
    }
}

const fn image_variant_settings_to_domain(value: ImageVariantSettingsDto) -> ImageVariantSettings {
    ImageVariantSettings {
        thumbnail_long_edge: value.thumbnail_long_edge,
        preview_long_edge: value.preview_long_edge,
    }
}
