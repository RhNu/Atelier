use super::{
    Character, CharacterPosition, CharacterReference, DatabaseResult, Deserialize,
    GenerateImageRequest, GenerateImageStreamRequest, GenerationWorkRequest, ImageSize,
    Img2ImgRequest, InpaintRequest, Serialize, VibeReference, VibeTransferConfig,
    character_reference_type_as_str, character_reference_type_from_str, image_format_as_str,
    image_format_from_str, image_model_as_str, image_model_from_str, noise_schedule_as_str,
    noise_schedule_from_str, quality_preset_as_str, quality_preset_from_str, sampler_as_str,
    sampler_from_str, stream_mode_as_str, stream_mode_from_str, uc_preset_as_str,
    uc_preset_from_str,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub(super) enum GenerationWorkRequestDto {
    Image(GenerateImageRequestDto),
    Stream(GenerateImageStreamRequestDto),
}

impl From<&GenerationWorkRequest> for GenerationWorkRequestDto {
    fn from(value: &GenerationWorkRequest) -> Self {
        match value {
            GenerationWorkRequest::Image(request) => {
                Self::Image(GenerateImageRequestDto::from(request))
            }
            GenerationWorkRequest::Stream(request) => {
                Self::Stream(GenerateImageStreamRequestDto::from(request))
            }
        }
    }
}

impl GenerationWorkRequestDto {
    pub(super) fn into_domain(self) -> DatabaseResult<GenerationWorkRequest> {
        match self {
            Self::Image(request) => Ok(GenerationWorkRequest::Image(request.into_domain()?)),
            Self::Stream(request) => Ok(GenerationWorkRequest::Stream(request.into_domain()?)),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct GenerateImageStreamRequestDto {
    base: GenerateImageRequestDto,
    stream: String,
}

impl From<&GenerateImageStreamRequest> for GenerateImageStreamRequestDto {
    fn from(value: &GenerateImageStreamRequest) -> Self {
        Self {
            base: GenerateImageRequestDto::from(&value.base),
            stream: stream_mode_as_str(value.stream).to_owned(),
        }
    }
}

impl GenerateImageStreamRequestDto {
    pub(super) fn into_domain(self) -> DatabaseResult<GenerateImageStreamRequest> {
        Ok(GenerateImageStreamRequest {
            base: self.base.into_domain()?,
            stream: stream_mode_from_str(&self.stream)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct GenerateImageRequestDto {
    prompt: String,
    #[serde(default)]
    furry_mode: bool,
    model: String,
    width: u32,
    height: u32,
    negative_prompt: Option<String>,
    quality: String,
    transparent_background: bool,
    uc_preset: String,
    steps: u32,
    scale: f32,
    sampler: String,
    noise_schedule: String,
    seed: i64,
    n_samples: u32,
    cfg_rescale: f32,
    variety_boost: bool,
    img2img: Option<Img2ImgRequestDto>,
    vibe_transfer: Option<VibeTransferConfigDto>,
    character_references: Option<Vec<CharacterReferenceDto>>,
    characters: Option<Vec<CharacterDto>>,
    use_coords: Option<bool>,
    image_format: Option<String>,
    strict_mode: bool,
}

impl From<&GenerateImageRequest> for GenerateImageRequestDto {
    fn from(value: &GenerateImageRequest) -> Self {
        Self {
            prompt: value.prompt.clone(),
            furry_mode: value.furry_mode,
            model: image_model_as_str(value.model).to_owned(),
            width: value.size.width,
            height: value.size.height,
            negative_prompt: value.negative_prompt.clone(),
            quality: quality_preset_as_str(value.quality).to_owned(),
            transparent_background: value.transparent_background,
            uc_preset: uc_preset_as_str(value.uc_preset).to_owned(),
            steps: value.steps,
            scale: value.scale,
            sampler: sampler_as_str(value.sampler).to_owned(),
            noise_schedule: noise_schedule_as_str(value.noise_schedule).to_owned(),
            seed: value.seed,
            n_samples: value.n_samples,
            cfg_rescale: value.cfg_rescale,
            variety_boost: value.variety_boost,
            img2img: value.img2img.as_ref().map(Img2ImgRequestDto::from),
            vibe_transfer: value
                .vibe_transfer
                .as_ref()
                .map(VibeTransferConfigDto::from),
            character_references: value
                .character_references
                .as_ref()
                .map(|items| items.iter().map(CharacterReferenceDto::from).collect()),
            characters: value
                .characters
                .as_ref()
                .map(|items| items.iter().map(CharacterDto::from).collect()),
            use_coords: value.use_coords,
            image_format: value
                .image_format
                .map(image_format_as_str)
                .map(str::to_owned),
            strict_mode: value.strict_mode,
        }
    }
}

impl GenerateImageRequestDto {
    pub(super) fn into_domain(self) -> DatabaseResult<GenerateImageRequest> {
        Ok(GenerateImageRequest {
            prompt: self.prompt,
            furry_mode: self.furry_mode,
            model: image_model_from_str(&self.model)?,
            size: ImageSize {
                width: self.width,
                height: self.height,
            },
            negative_prompt: self.negative_prompt,
            quality: quality_preset_from_str(&self.quality)?,
            transparent_background: self.transparent_background,
            uc_preset: uc_preset_from_str(&self.uc_preset)?,
            steps: self.steps,
            scale: self.scale,
            sampler: sampler_from_str(&self.sampler)?,
            noise_schedule: noise_schedule_from_str(&self.noise_schedule)?,
            seed: self.seed,
            n_samples: self.n_samples,
            cfg_rescale: self.cfg_rescale,
            variety_boost: self.variety_boost,
            img2img: self.img2img.map(Into::into),
            vibe_transfer: self.vibe_transfer.map(Into::into),
            character_references: self
                .character_references
                .map(|items| {
                    items
                        .into_iter()
                        .map(CharacterReferenceDto::into_domain)
                        .collect::<DatabaseResult<Vec<_>>>()
                })
                .transpose()?,
            characters: self
                .characters
                .map(|items| items.into_iter().map(Into::into).collect()),
            use_coords: self.use_coords,
            image_format: self
                .image_format
                .as_deref()
                .map(image_format_from_str)
                .transpose()?,
            strict_mode: self.strict_mode,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Img2ImgRequestDto {
    image: String,
    strength: f32,
    noise: f32,
    mask: Option<String>,
}

impl From<&Img2ImgRequest> for Img2ImgRequestDto {
    fn from(value: &Img2ImgRequest) -> Self {
        Self {
            image: value.image.clone(),
            strength: value.strength,
            noise: value.noise,
            mask: value
                .inpaint
                .as_ref()
                .map(|inpaint| inpaint.region_to_replace.clone()),
        }
    }
}

impl From<Img2ImgRequestDto> for Img2ImgRequest {
    fn from(value: Img2ImgRequestDto) -> Self {
        Self {
            image: value.image,
            strength: value.strength,
            noise: value.noise,
            inpaint: value
                .mask
                .map(|region_to_replace| InpaintRequest { region_to_replace }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct VibeReferenceDto {
    vibe_data_cache: String,
    strength: f32,
}

impl From<&VibeReference> for VibeReferenceDto {
    fn from(value: &VibeReference) -> Self {
        Self {
            vibe_data_cache: value.vibe_data_cache.clone(),
            strength: value.strength,
        }
    }
}

impl From<VibeReferenceDto> for VibeReference {
    fn from(value: VibeReferenceDto) -> Self {
        Self {
            vibe_data_cache: value.vibe_data_cache,
            strength: value.strength,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct VibeTransferConfigDto {
    references: Vec<VibeReferenceDto>,
    strength: f32,
}

impl From<&VibeTransferConfig> for VibeTransferConfigDto {
    fn from(value: &VibeTransferConfig) -> Self {
        Self {
            references: value
                .references
                .iter()
                .map(VibeReferenceDto::from)
                .collect(),
            strength: value.strength,
        }
    }
}

impl From<VibeTransferConfigDto> for VibeTransferConfig {
    fn from(value: VibeTransferConfigDto) -> Self {
        Self {
            references: value.references.into_iter().map(Into::into).collect(),
            strength: value.strength,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct CharacterReferenceDto {
    image: String,
    reference_type: String,
    fidelity: f32,
    strength: f32,
}

impl From<&CharacterReference> for CharacterReferenceDto {
    fn from(value: &CharacterReference) -> Self {
        Self {
            image: value.image.clone(),
            reference_type: character_reference_type_as_str(value.reference_type).to_owned(),
            fidelity: value.fidelity,
            strength: value.strength,
        }
    }
}

impl CharacterReferenceDto {
    pub(super) fn into_domain(self) -> DatabaseResult<CharacterReference> {
        Ok(CharacterReference {
            image: self.image,
            reference_type: character_reference_type_from_str(&self.reference_type)?,
            fidelity: self.fidelity,
            strength: self.strength,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct CharacterDto {
    prompt: String,
    negative_prompt: Option<String>,
    x: f32,
    y: f32,
    enabled: bool,
}

impl From<&Character> for CharacterDto {
    fn from(value: &Character) -> Self {
        Self {
            prompt: value.prompt.clone(),
            negative_prompt: value.negative_prompt.clone(),
            x: value.position.x,
            y: value.position.y,
            enabled: value.enabled,
        }
    }
}

impl From<CharacterDto> for Character {
    fn from(value: CharacterDto) -> Self {
        Self {
            prompt: value.prompt,
            negative_prompt: value.negative_prompt,
            position: CharacterPosition {
                x: value.x,
                y: value.y,
            },
            enabled: value.enabled,
        }
    }
}
