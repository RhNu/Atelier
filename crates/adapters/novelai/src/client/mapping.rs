use super::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, DirectorTool,
    EncodeVibeRequest, GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage,
    GeneratedImageMetadata, GeneratedImageMetadataWarning, ImageFormat, ImageSize,
    ImageStreamEvent, Img2ImgRequest, NoiseSchedule, NovelAiBridgeError,
    ParsedGeneratedImageMetadata, QualityPreset, RunDirectorToolRequest, Sampler, SecretsError,
    StreamMode, SubscriptionSummary, UcPreset, V5UsageStatus, VibeModel, VibeTransferConfig,
    bridge,
};

pub(super) fn map_secrets_error(error: &SecretsError) -> NovelAiBridgeError {
    NovelAiBridgeError::credential(error.to_string())
}

pub(super) fn to_bridge_generate_request(
    request: GenerateImageRequest,
) -> Result<bridge::GenerateImageRequest, bridge::BridgeError> {
    let references = bridge::ReferenceSet {
        vibe_transfer: request
            .vibe_transfer
            .map(to_bridge_vibe_transfer)
            .transpose()?,
        character_references: request
            .character_references
            .unwrap_or_default()
            .into_iter()
            .map(|reference| to_bridge_character_reference(&reference))
            .collect::<Result<_, _>>()?,
    };
    Ok(bridge::GenerateImageRequest {
        core: bridge::GenerationCore {
            prompt: request.prompt,
            furry_mode: false,
            negative_prompt: request.negative_prompt,
            model: request.model.bridge_model(),
            size: to_bridge_size(request.size),
            seed: request.seed,
            n_samples: request.n_samples,
            quality: to_bridge_quality(request.quality),
            transparent_background: request.transparent_background,
            uc_preset: to_bridge_uc_preset(request.uc_preset),
        },
        sampling: bridge::SamplingParams {
            steps: request.steps,
            scale: request.scale,
            sampler: to_bridge_sampler(request.sampler),
            noise_schedule: to_bridge_noise_schedule(request.noise_schedule),
            cfg_rescale: request.cfg_rescale,
        },
        characters: request.characters.map(|items| bridge::CharacterSet {
            characters: items.into_iter().map(to_bridge_character).collect(),
            use_coords: request.use_coords,
        }),
        guidance: bridge::GuidanceParams {
            variety_boost: request.variety_boost,
            ..bridge::GuidanceParams::default()
        },
        img2img: request.img2img.map(to_bridge_i2i).transpose()?,
        references: (!references.is_empty()).then_some(references),
        output: bridge::OutputParams {
            image_format: request.image_format.map(to_bridge_image_format),
        },
        strict_mode: request.strict_mode,
    })
}

pub(super) fn to_bridge_stream_request(
    request: GenerateImageStreamRequest,
) -> Result<bridge::GenerateImageStreamRequest, bridge::BridgeError> {
    Ok(bridge::GenerateImageStreamRequest {
        base: to_bridge_generate_request(request.base)?,
        stream: to_bridge_stream_mode(request.stream),
    })
}

pub(super) fn to_bridge_encode_vibe_request(
    request: &EncodeVibeRequest,
) -> Result<bridge::EncodeVibeRequest, bridge::BridgeError> {
    Ok(bridge::EncodeVibeRequest {
        image: bridge::EncodedImage::from_base64(&request.image)?,
        information_extracted: request.information_extracted,
        model: to_bridge_vibe_model(request.model),
        strict_mode: request.strict_mode,
    })
}

pub(super) fn to_bridge_director_request(
    request: RunDirectorToolRequest,
) -> Result<bridge::RunDirectorToolRequest, bridge::BridgeError> {
    let command = match request.tool {
        DirectorTool::Lineart => bridge::DirectorCommand::Lineart,
        DirectorTool::Sketch => bridge::DirectorCommand::Sketch,
        DirectorTool::BgRemoval => bridge::DirectorCommand::BgRemoval,
        DirectorTool::Emotion => bridge::DirectorCommand::Emotion {
            prompt: request.prompt.expect("normalized emotion prompt"),
            defry: request.defry.expect("normalized emotion defry"),
        },
        DirectorTool::Declutter => bridge::DirectorCommand::Declutter,
        DirectorTool::Colorize => bridge::DirectorCommand::Colorize {
            prompt: request.prompt,
            defry: request.defry.expect("normalized colorize defry"),
        },
    };
    Ok(bridge::RunDirectorToolRequest {
        image: bridge::EncodedImage::from_base64(&request.image)?,
        command,
        strict_mode: request.strict_mode,
    })
}

pub(super) fn from_bridge_generated_image(image: bridge::GeneratedImage) -> GeneratedImage {
    GeneratedImage {
        bytes: image.bytes,
        mime_type: image.mime_type,
        metadata: from_bridge_generated_image_metadata(image.metadata),
    }
}

pub(super) fn from_bridge_generated_image_metadata(
    metadata: bridge::GeneratedImageMetadata,
) -> GeneratedImageMetadata {
    match metadata {
        bridge::GeneratedImageMetadata::Parsed(metadata) => {
            let metadata_json = match serde_json::to_string(metadata.as_ref()) {
                Ok(value) => value,
                Err(error) => {
                    return GeneratedImageMetadata::Invalid {
                        message: error.to_string(),
                    };
                }
            };
            GeneratedImageMetadata::Parsed(ParsedGeneratedImageMetadata {
                prompt: metadata.prompt.clone(),
                negative_prompt: metadata.negative_prompt.clone(),
                seed: metadata.seed,
                metadata_json,
                warnings: metadata
                    .warnings
                    .iter()
                    .map(|warning| match warning {
                        bridge::PngMetadataWarning::InvalidCommentJson => {
                            GeneratedImageMetadataWarning::InvalidCommentJson
                        }
                        bridge::PngMetadataWarning::InvalidTextChunk { keyword, message } => {
                            GeneratedImageMetadataWarning::InvalidTextChunk {
                                keyword: keyword.clone(),
                                message: message.clone(),
                            }
                        }
                    })
                    .collect(),
            })
        }
        bridge::GeneratedImageMetadata::NotPresent => GeneratedImageMetadata::NotPresent,
        bridge::GeneratedImageMetadata::UnsupportedFormat => {
            GeneratedImageMetadata::UnsupportedFormat
        }
        bridge::GeneratedImageMetadata::Invalid { message } => {
            GeneratedImageMetadata::Invalid { message }
        }
    }
}

pub(super) fn from_bridge_stream_chunk(chunk: bridge::ImageStreamChunk) -> ImageStreamEvent {
    ImageStreamEvent {
        event_type: chunk.event_type,
        sample_index: chunk.samp_ix,
        step_index: chunk.step_ix,
        generation_id: chunk.gen_id,
        sigma: chunk.sigma,
        image: chunk.image,
    }
}

pub(super) fn from_bridge_subscription(
    subscription: bridge::SubscriptionInfo,
) -> SubscriptionSummary {
    SubscriptionSummary {
        anlas_balance: subscription.anlas_balance,
        is_opus: subscription.is_opus,
        subscription_active: subscription.subscription_active,
        tier: subscription.tier,
        tier_name: subscription.tier_name,
        expires_at_ms: subscription.expires_at_ms,
        v5_usage: subscription.v5_usage.map(|usage| V5UsageStatus {
            is_negative: usage.is_negative,
            percent: usage.percent,
            seconds_until_next_percent: usage.seconds_until_next_percent,
        }),
    }
}

pub(super) const fn to_bridge_vibe_model(model: VibeModel) -> bridge::Model {
    match model {
        VibeModel::NaiDiffusion5Full => bridge::Model::NaiDiffusion5Full,
        VibeModel::NaiDiffusion5Curated => bridge::Model::NaiDiffusion5Curated,
        VibeModel::NaiDiffusion45Full => bridge::Model::NaiDiffusion45Full,
        VibeModel::NaiDiffusion45Curated => bridge::Model::NaiDiffusion45Curated,
        VibeModel::NaiDiffusion4Full => bridge::Model::NaiDiffusion4Full,
        VibeModel::NaiDiffusion4Curated => bridge::Model::NaiDiffusion4Curated,
        VibeModel::NaiDiffusion3 => bridge::Model::NaiDiffusion3,
        VibeModel::NaiDiffusion3Furry => bridge::Model::NaiDiffusion3Furry,
    }
}

pub(super) const fn to_bridge_size(size: ImageSize) -> bridge::ImageSize {
    bridge::ImageSize {
        width: size.width,
        height: size.height,
    }
}

pub(super) const fn to_bridge_sampler(sampler: Sampler) -> bridge::Sampler {
    match sampler {
        Sampler::KEuler => bridge::Sampler::KEuler,
        Sampler::KEulerAncestral => bridge::Sampler::KEulerAncestral,
        Sampler::KDpm2 => bridge::Sampler::KDpm2,
        Sampler::KDpm2Ancestral => bridge::Sampler::KDpm2Ancestral,
        Sampler::KDpmpp2m => bridge::Sampler::KDpmpp2m,
        Sampler::KDpmpp2mSde => bridge::Sampler::KDpmpp2mSde,
        Sampler::KDpmpp2sAncestral => bridge::Sampler::KDpmpp2sAncestral,
        Sampler::KDpmppSde => bridge::Sampler::KDpmppSde,
        Sampler::Ddim => bridge::Sampler::Ddim,
        Sampler::DdimV3 => bridge::Sampler::DdimV3,
    }
}

pub(super) const fn to_bridge_noise_schedule(schedule: NoiseSchedule) -> bridge::NoiseSchedule {
    match schedule {
        NoiseSchedule::Native => bridge::NoiseSchedule::Native,
        NoiseSchedule::Karras => bridge::NoiseSchedule::Karras,
        NoiseSchedule::Exponential => bridge::NoiseSchedule::Exponential,
        NoiseSchedule::Polyexponential => bridge::NoiseSchedule::Polyexponential,
    }
}

pub(super) const fn to_bridge_uc_preset(preset: UcPreset) -> bridge::UcPreset {
    match preset {
        UcPreset::Heavy => bridge::UcPreset::Heavy,
        UcPreset::Light => bridge::UcPreset::Light,
        UcPreset::FurryFocus => bridge::UcPreset::FurryFocus,
        UcPreset::HumanFocus => bridge::UcPreset::HumanFocus,
        UcPreset::None => bridge::UcPreset::None,
    }
}

pub(super) const fn to_bridge_image_format(format: ImageFormat) -> bridge::ImageFormat {
    match format {
        ImageFormat::Png => bridge::ImageFormat::Png,
        ImageFormat::Webp => bridge::ImageFormat::Webp,
    }
}

pub(super) const fn to_bridge_stream_mode(mode: StreamMode) -> bridge::StreamMode {
    match mode {
        StreamMode::Sse => bridge::StreamMode::Sse,
    }
}

pub(super) fn to_bridge_i2i(
    request: Img2ImgRequest,
) -> Result<bridge::Img2ImgRequest, bridge::BridgeError> {
    let image = bridge::EncodedImage::from_base64(&request.image)?;
    let mut result = bridge::Img2ImgRequest::new(image, request.strength, request.noise);
    result.mask = request
        .mask
        .map(|mask| bridge::EncodedImage::from_base64(&mask))
        .transpose()?;
    Ok(result)
}

pub(super) fn to_bridge_vibe_transfer(
    config: VibeTransferConfig,
) -> Result<bridge::VibeTransferConfig, bridge::BridgeError> {
    Ok(bridge::VibeTransferConfig {
        references: config
            .references
            .into_iter()
            .map(|reference| {
                Ok(bridge::VibeReference {
                    vibe_data_cache: bridge::VibeEncoding::from_base64(&reference.vibe_data_cache)?,
                    strength: reference.strength,
                })
            })
            .collect::<Result<_, bridge::BridgeError>>()?,
        strength: config.strength,
    })
}

pub(super) const fn to_bridge_character_position(
    position: CharacterPosition,
) -> bridge::CharacterPosition {
    bridge::CharacterPosition {
        x: position.x,
        y: position.y,
    }
}

pub(super) fn to_bridge_character(character: Character) -> bridge::Character {
    bridge::Character {
        prompt: character.prompt,
        negative_prompt: character.negative_prompt,
        position: to_bridge_character_position(character.position),
        enabled: character.enabled,
    }
}

pub(super) const fn to_bridge_character_reference_type(
    reference_type: CharacterReferenceType,
) -> bridge::CharacterReferenceType {
    match reference_type {
        CharacterReferenceType::Character => bridge::CharacterReferenceType::Character,
        CharacterReferenceType::Style => bridge::CharacterReferenceType::Style,
        CharacterReferenceType::CharacterAndStyle => {
            bridge::CharacterReferenceType::CharacterAndStyle
        }
        CharacterReferenceType::Costume => bridge::CharacterReferenceType::Costume,
        CharacterReferenceType::Delta => bridge::CharacterReferenceType::Delta,
    }
}

pub(super) const fn to_bridge_quality(quality: QualityPreset) -> bridge::QualityPreset {
    match quality {
        QualityPreset::Standard => bridge::QualityPreset::Standard,
        QualityPreset::Light => bridge::QualityPreset::Light,
        QualityPreset::None => bridge::QualityPreset::None,
    }
}

pub(super) fn to_bridge_character_reference(
    reference: &CharacterReference,
) -> Result<bridge::CharacterReference, bridge::BridgeError> {
    Ok(bridge::CharacterReference {
        image: bridge::EncodedImage::from_base64(&reference.image)?,
        reference_type: to_bridge_character_reference_type(reference.reference_type),
        fidelity: reference.fidelity,
        strength: reference.strength,
    })
}
