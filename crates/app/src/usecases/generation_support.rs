use super::{
    AnlasEstimate, AppError, AppResult, CharacterReference, ControlNetConfig, ControlNetInput,
    GenerateImageRequest, GenerateImageRequestDto, GenerationAnlasEstimateDto,
    GenerationEstimateRequestDto, ImageSize, Img2ImgRequest, RunHistoryRepository, UcPresetDto,
    character_reference_type_to_domain, characters_to_domain, image_format_to_domain,
    image_model_to_domain, noise_schedule_to_domain, plan_context_to_domain,
    plan_generation_request, sampler_to_domain, uc_preset_to_domain,
};

pub(super) async fn ensure_generation_batch_target_is_new<'a, R, I>(
    repository: &R,
    batch_id: &str,
    job_ids: I,
) -> AppResult<()>
where
    R: RunHistoryRepository,
    I: IntoIterator<Item = &'a str>,
{
    let job_ids = job_ids.into_iter().collect::<Vec<_>>();
    if job_ids.is_empty() {
        return Err(AppError::new(
            "invalid_request",
            "generation batch requires at least one job",
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    for job_id in &job_ids {
        if !unique.insert(*job_id) {
            return Err(AppError::new(
                "invalid_request",
                "generation batch contains duplicate job_id",
            ));
        }
    }
    if repository
        .run_history_batch_exists(batch_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?
    {
        return Err(AppError::new(
            "invalid_request",
            "generation batch_id already exists in run history",
        ));
    }
    for job_id in job_ids {
        if repository
            .get_run_history(job_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?
            .is_some()
        {
            return Err(AppError::new(
                "invalid_request",
                "generation job_id already exists in run history",
            ));
        }
    }
    Ok(())
}

const fn anlas_estimate_to_dto(value: AnlasEstimate) -> GenerationAnlasEstimateDto {
    GenerationAnlasEstimateDto {
        per_sample_cost: value.per_sample_cost,
        per_request_cost: value.per_request_cost,
        total_cost: value.total_cost,
        adjusted_resolution: value.adjusted_resolution,
        opus_discount_applied: value.opus_discount_applied,
        pending_encode_cost: value.pending_encode_cost,
    }
}

pub(super) fn estimate_generation_anlas(
    request: &GenerationEstimateRequestDto,
) -> AppResult<GenerationAnlasEstimateDto> {
    let plan = plan_generation_request(
        estimate_request_to_domain(&request.request),
        plan_context_to_domain(request.context),
    )
    .map_err(|error| AppError::new("invalid_request", error.to_string()))?;
    Ok(anlas_estimate_to_dto(plan.anlas_estimate))
}

fn estimate_request_to_domain(value: &GenerateImageRequestDto) -> GenerateImageRequest {
    GenerateImageRequest {
        prompt: if value.prompt.trim().is_empty() {
            "estimate".to_owned()
        } else {
            value.prompt.clone()
        },
        model: image_model_to_domain(value.model),
        size: ImageSize {
            width: value.size.width,
            height: value.size.height,
        },
        negative_prompt: value.negative_prompt.clone(),
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
        i2i: value.i2i.as_ref().map(|i2i| Img2ImgRequest {
            image: "estimate".to_owned(),
            strength: i2i.strength,
            noise: i2i.noise,
            mask: i2i.mask.as_ref().map(|_| "estimate".to_owned()),
        }),
        controlnet: value
            .controlnet
            .as_ref()
            .map(|controlnet| ControlNetConfig {
                images: controlnet
                    .images
                    .iter()
                    .map(|image| ControlNetInput {
                        vibe_data_cache: "estimate".to_owned(),
                        info_extracted: image.info_extracted,
                        strength: image.strength,
                    })
                    .collect(),
                strength: controlnet.strength,
            }),
        character_references: value.character_references.as_ref().map(|references| {
            references
                .iter()
                .map(|reference| CharacterReference {
                    image: "estimate".to_owned(),
                    reference_type: character_reference_type_to_domain(reference.reference_type),
                    fidelity: reference.fidelity,
                    strength: reference.strength,
                })
                .collect()
        }),
        characters: value.characters.clone().map(characters_to_domain),
        use_coords: value.use_coords,
        image_format: value.image_format.map(image_format_to_domain),
        strict_mode: value.strict_mode,
    }
}

pub(super) fn quality_override_to_bool(value: &str) -> bool {
    !matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "" | "false" | "off" | "none" | "disabled"
    )
}

pub(super) fn parse_uc_preset_override(value: &str) -> AppResult<UcPresetDto> {
    match value.trim().to_ascii_lowercase().as_str() {
        "heavy" => Ok(UcPresetDto::Heavy),
        "light" => Ok(UcPresetDto::Light),
        "furry_focus" | "furry-focus" | "furry focus" => Ok(UcPresetDto::FurryFocus),
        "human_focus" | "human-focus" | "human focus" => Ok(UcPresetDto::HumanFocus),
        "none" => Ok(UcPresetDto::None),
        other => Err(AppError::new(
            "prompt_invalid_request",
            format!("unknown uc preset override `{other}`"),
        )),
    }
}
