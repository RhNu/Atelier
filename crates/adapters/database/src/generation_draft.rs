use async_trait::async_trait;
use atelier_generation::{
    CharacterPosition, GenerationDraftCharacter, GenerationDraftCharacterPositionMode,
    GenerationDraftError, GenerationDraftI2i, GenerationDraftPreciseReference,
    GenerationDraftRepository, GenerationDraftResult, GenerationDraftSeedMode,
    GenerationDraftSnapshot, GenerationDraftVibe, GenerationDraftVibeSlot, ImageSize,
};
use atelier_resource_catalog::{ResourceId, ResourceRef, VariantId};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::codec::{decode_json, encode_json};
use crate::generation_codec::scalars::{
    character_reference_type_as_str, character_reference_type_from_str, image_format_as_str,
    image_format_from_str, image_model_as_str, image_model_from_str, noise_schedule_as_str,
    noise_schedule_from_str, sampler_as_str, sampler_from_str, uc_preset_as_str,
    uc_preset_from_str,
};
use crate::{DatabaseConnection, DatabaseError};

const DRAFT_KEY: &str = "generation.draft";
const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct DatabaseGenerationDraftRepository {
    connection: DatabaseConnection,
}

impl DatabaseGenerationDraftRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl GenerationDraftRepository for DatabaseGenerationDraftRepository {
    async fn load_generation_draft(
        &self,
    ) -> GenerationDraftResult<Option<GenerationDraftSnapshot>> {
        let json = {
            let connection = self.connection.lock().map_err(draft_database_error)?;
            connection
                .query_row(
                    "SELECT value_json FROM workspace_settings WHERE setting_key = ?1",
                    params![DRAFT_KEY],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(draft_sql_error)?
        };
        json.as_deref()
            .map(GenerationDraftDto::decode_domain)
            .transpose()
    }

    async fn save_generation_draft(
        &self,
        draft: &GenerationDraftSnapshot,
    ) -> GenerationDraftResult<()> {
        let json = GenerationDraftDto::encode_domain(draft)?;
        let connection = self.connection.lock().map_err(draft_database_error)?;
        connection
            .execute(
                r"
                INSERT INTO workspace_settings(setting_key, value_json)
                VALUES (?1, ?2)
                ON CONFLICT(setting_key) DO UPDATE SET value_json = excluded.value_json
                ",
                params![DRAFT_KEY, json],
            )
            .map(|_| ())
            .map_err(draft_sql_error)
    }

    async fn clear_generation_draft(&self) -> GenerationDraftResult<()> {
        let connection = self.connection.lock().map_err(draft_database_error)?;
        connection
            .execute(
                "DELETE FROM workspace_settings WHERE setting_key = ?1",
                params![DRAFT_KEY],
            )
            .map(|_| ())
            .map_err(draft_sql_error)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::struct_excessive_bools)]
struct GenerationDraftDto {
    schema_version: u32,
    main_preset_id: Option<String>,
    prompt: String,
    negative_prompt: String,
    model: String,
    width: u32,
    height: u32,
    quality: bool,
    uc_preset: String,
    steps: u32,
    scale: f32,
    sampler: String,
    noise_schedule: String,
    seed_mode: String,
    seed: i64,
    n_samples: u32,
    request_count: u32,
    cfg_rescale: f32,
    variety_boost: bool,
    image_format: Option<String>,
    strict_mode: bool,
    stream_enabled: bool,
    i2i: Option<GenerationDraftI2iDto>,
    vibe: GenerationDraftVibeDto,
    precise_references: Vec<GenerationDraftPreciseReferenceDto>,
    characters: Vec<GenerationDraftCharacterDto>,
    character_position_mode: String,
}

impl GenerationDraftDto {
    fn from_domain(value: &GenerationDraftSnapshot) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            main_preset_id: value.main_preset_id.clone(),
            prompt: value.prompt.clone(),
            negative_prompt: value.negative_prompt.clone(),
            model: image_model_as_str(value.model).to_owned(),
            width: value.size.width,
            height: value.size.height,
            quality: value.quality,
            uc_preset: uc_preset_as_str(value.uc_preset).to_owned(),
            steps: value.steps,
            scale: value.scale,
            sampler: sampler_as_str(value.sampler).to_owned(),
            noise_schedule: noise_schedule_as_str(value.noise_schedule).to_owned(),
            seed_mode: draft_seed_mode_as_str(value.seed_mode).to_owned(),
            seed: value.seed,
            n_samples: value.n_samples,
            request_count: value.request_count,
            cfg_rescale: value.cfg_rescale,
            variety_boost: value.variety_boost,
            image_format: value
                .image_format
                .map(image_format_as_str)
                .map(str::to_owned),
            strict_mode: value.strict_mode,
            stream_enabled: value.stream_enabled,
            i2i: value.i2i.as_ref().map(GenerationDraftI2iDto::from_domain),
            vibe: GenerationDraftVibeDto::from_domain(&value.vibe),
            precise_references: value
                .precise_references
                .iter()
                .map(GenerationDraftPreciseReferenceDto::from_domain)
                .collect(),
            characters: value
                .characters
                .iter()
                .map(GenerationDraftCharacterDto::from_domain)
                .collect(),
            character_position_mode: position_mode_as_str(value.character_position_mode).to_owned(),
        }
    }

    fn into_domain(self) -> GenerationDraftResult<GenerationDraftSnapshot> {
        ensure_schema(self.schema_version)?;
        Ok(GenerationDraftSnapshot {
            main_preset_id: self.main_preset_id,
            prompt: self.prompt,
            negative_prompt: self.negative_prompt,
            model: map_database(image_model_from_str(&self.model))?,
            size: ImageSize {
                width: self.width,
                height: self.height,
            },
            quality: self.quality,
            uc_preset: map_database(uc_preset_from_str(&self.uc_preset))?,
            steps: self.steps,
            scale: self.scale,
            sampler: map_database(sampler_from_str(&self.sampler))?,
            noise_schedule: map_database(noise_schedule_from_str(&self.noise_schedule))?,
            seed_mode: draft_seed_mode_from_str(&self.seed_mode)?,
            seed: self.seed,
            n_samples: self.n_samples,
            request_count: self.request_count,
            cfg_rescale: self.cfg_rescale,
            variety_boost: self.variety_boost,
            image_format: self
                .image_format
                .as_deref()
                .map(image_format_from_str)
                .transpose()
                .map_err(draft_database_error)?,
            strict_mode: self.strict_mode,
            stream_enabled: self.stream_enabled,
            i2i: self.i2i.map(GenerationDraftI2iDto::into_domain),
            vibe: self.vibe.into_domain(),
            precise_references: self
                .precise_references
                .into_iter()
                .map(GenerationDraftPreciseReferenceDto::into_domain)
                .collect::<GenerationDraftResult<_>>()?,
            characters: self
                .characters
                .into_iter()
                .map(GenerationDraftCharacterDto::into_domain)
                .collect(),
            character_position_mode: position_mode_from_str(&self.character_position_mode)?,
        })
    }

    fn encode_domain(value: &GenerationDraftSnapshot) -> GenerationDraftResult<String> {
        encode_json(&Self::from_domain(value)).map_err(draft_database_error)
    }

    fn decode_domain(value: &str) -> GenerationDraftResult<GenerationDraftSnapshot> {
        decode_json::<Self>(value)
            .map_err(draft_database_error)?
            .into_domain()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationDraftI2iDto {
    image: ResourceRefDto,
    mask: Option<ResourceRefDto>,
    strength: f32,
    noise: f32,
}

impl GenerationDraftI2iDto {
    fn from_domain(value: &GenerationDraftI2i) -> Self {
        Self {
            image: ResourceRefDto::from_domain(&value.image),
            mask: value.mask.as_ref().map(ResourceRefDto::from_domain),
            strength: value.strength,
            noise: value.noise,
        }
    }

    fn into_domain(self) -> GenerationDraftI2i {
        GenerationDraftI2i {
            image: self.image.into_domain(),
            mask: self.mask.map(ResourceRefDto::into_domain),
            strength: self.strength,
            noise: self.noise,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationDraftVibeDto {
    enabled: bool,
    strength: f32,
    slots: Vec<GenerationDraftVibeSlotDto>,
}

impl GenerationDraftVibeDto {
    fn from_domain(value: &GenerationDraftVibe) -> Self {
        Self {
            enabled: value.enabled,
            strength: value.strength,
            slots: value
                .slots
                .iter()
                .map(GenerationDraftVibeSlotDto::from_domain)
                .collect(),
        }
    }

    fn into_domain(self) -> GenerationDraftVibe {
        GenerationDraftVibe {
            enabled: self.enabled,
            strength: self.strength,
            slots: self
                .slots
                .into_iter()
                .map(GenerationDraftVibeSlotDto::into_domain)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationDraftVibeSlotDto {
    id: String,
    encoding: ResourceRefDto,
    vibe_id: Option<String>,
    information_extracted: f32,
    strength: f32,
    display_name: String,
    source_image: Option<ResourceRefDto>,
    source_sha256: Option<String>,
}

impl GenerationDraftVibeSlotDto {
    fn from_domain(value: &GenerationDraftVibeSlot) -> Self {
        Self {
            id: value.id.clone(),
            encoding: ResourceRefDto::from_domain(&value.encoding),
            vibe_id: value.vibe_id.clone(),
            information_extracted: value.information_extracted,
            strength: value.strength,
            display_name: value.display_name.clone(),
            source_image: value.source_image.as_ref().map(ResourceRefDto::from_domain),
            source_sha256: value.source_sha256.clone(),
        }
    }

    fn into_domain(self) -> GenerationDraftVibeSlot {
        GenerationDraftVibeSlot {
            id: self.id,
            encoding: self.encoding.into_domain(),
            vibe_id: self.vibe_id,
            information_extracted: self.information_extracted,
            strength: self.strength,
            display_name: self.display_name,
            source_image: self.source_image.map(ResourceRefDto::into_domain),
            source_sha256: self.source_sha256,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationDraftPreciseReferenceDto {
    id: String,
    image: ResourceRefDto,
    reference_type: String,
    fidelity: f32,
    strength: f32,
    display_name: String,
}

impl GenerationDraftPreciseReferenceDto {
    fn from_domain(value: &GenerationDraftPreciseReference) -> Self {
        Self {
            id: value.id.clone(),
            image: ResourceRefDto::from_domain(&value.image),
            reference_type: character_reference_type_as_str(value.reference_type).to_owned(),
            fidelity: value.fidelity,
            strength: value.strength,
            display_name: value.display_name.clone(),
        }
    }

    fn into_domain(self) -> GenerationDraftResult<GenerationDraftPreciseReference> {
        Ok(GenerationDraftPreciseReference {
            id: self.id,
            image: self.image.into_domain(),
            reference_type: map_database(character_reference_type_from_str(&self.reference_type))?,
            fidelity: self.fidelity,
            strength: self.strength,
            display_name: self.display_name,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationDraftCharacterDto {
    id: String,
    preset_id: Option<String>,
    prompt: String,
    negative_prompt: String,
    enabled: bool,
    x: f32,
    y: f32,
}

impl GenerationDraftCharacterDto {
    fn from_domain(value: &GenerationDraftCharacter) -> Self {
        Self {
            id: value.id.clone(),
            preset_id: value.preset_id.clone(),
            prompt: value.prompt.clone(),
            negative_prompt: value.negative_prompt.clone(),
            enabled: value.enabled,
            x: value.position.x,
            y: value.position.y,
        }
    }

    fn into_domain(self) -> GenerationDraftCharacter {
        GenerationDraftCharacter {
            id: self.id,
            preset_id: self.preset_id,
            prompt: self.prompt,
            negative_prompt: self.negative_prompt,
            enabled: self.enabled,
            position: CharacterPosition {
                x: self.x,
                y: self.y,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ResourceRefDto {
    id: String,
    variant_id: Option<String>,
}

impl ResourceRefDto {
    fn from_domain(value: &ResourceRef) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            variant_id: value.variant_id.as_ref().map(|id| id.as_str().to_owned()),
        }
    }

    fn into_domain(self) -> ResourceRef {
        ResourceRef::new(
            ResourceId::new(self.id),
            self.variant_id.map(VariantId::new),
        )
    }
}

const fn draft_seed_mode_as_str(value: GenerationDraftSeedMode) -> &'static str {
    match value {
        GenerationDraftSeedMode::Random => "random",
        GenerationDraftSeedMode::Fixed => "fixed",
    }
}

fn draft_seed_mode_from_str(value: &str) -> GenerationDraftResult<GenerationDraftSeedMode> {
    match value {
        "random" => Ok(GenerationDraftSeedMode::Random),
        "fixed" => Ok(GenerationDraftSeedMode::Fixed),
        _ => Err(GenerationDraftError::repository(format!(
            "unknown generation draft seed mode `{value}`"
        ))),
    }
}

const fn position_mode_as_str(value: GenerationDraftCharacterPositionMode) -> &'static str {
    match value {
        GenerationDraftCharacterPositionMode::Global => "global",
        GenerationDraftCharacterPositionMode::Manual => "manual",
    }
}

fn position_mode_from_str(
    value: &str,
) -> GenerationDraftResult<GenerationDraftCharacterPositionMode> {
    match value {
        "global" => Ok(GenerationDraftCharacterPositionMode::Global),
        "manual" => Ok(GenerationDraftCharacterPositionMode::Manual),
        _ => Err(GenerationDraftError::repository(format!(
            "unknown generation draft character position mode `{value}`"
        ))),
    }
}

fn ensure_schema(value: u32) -> GenerationDraftResult<()> {
    if value == JSON_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(GenerationDraftError::repository(format!(
            "unsupported generation draft schema version {value}"
        )))
    }
}

fn map_database<T>(result: Result<T, DatabaseError>) -> GenerationDraftResult<T> {
    result.map_err(draft_database_error)
}

fn draft_database_error(error: DatabaseError) -> GenerationDraftError {
    let message = error.to_string();
    drop(error);
    GenerationDraftError::repository(message)
}

fn draft_sql_error(error: rusqlite::Error) -> GenerationDraftError {
    let message = error.to_string();
    drop(error);
    GenerationDraftError::repository(message)
}
