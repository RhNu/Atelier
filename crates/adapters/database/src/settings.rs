#![allow(clippy::missing_const_for_fn)]

use async_trait::async_trait;
use atelier_generation::{ImageFormat, ImageModel, ImageSize, NoiseSchedule, Sampler, UcPreset};
use atelier_settings::{
    GenerationDefaults, ImageVariantSettings, SettingsError, SettingsResult, WorkspaceSettings,
    WorkspaceSettingsRepository,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::codec::{decode_json, encode_json};
use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;

const WORKSPACE_SETTINGS_KEY: &str = "workspace";
const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct DatabaseSettingsRepository {
    connection: DatabaseConnection,
}

impl DatabaseSettingsRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl WorkspaceSettingsRepository for DatabaseSettingsRepository {
    async fn get_workspace_settings(&self) -> SettingsResult<WorkspaceSettings> {
        let json = {
            let connection = self.connection.lock().map_err(settings_database_error)?;
            connection
                .query_row(
                    "SELECT value_json FROM workspace_settings WHERE setting_key = ?1",
                    params![WORKSPACE_SETTINGS_KEY],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(settings_sql_error)?
        };
        json.as_deref()
            .map(WorkspaceSettingsDto::decode_domain)
            .transpose()?
            .map_or_else(
                || Ok(WorkspaceSettings::default()),
                |settings| {
                    settings.validate()?;
                    Ok(settings)
                },
            )
    }

    async fn save_workspace_settings(&self, settings: WorkspaceSettings) -> SettingsResult<()> {
        settings.validate()?;
        let json = WorkspaceSettingsDto::encode_domain(&settings)?;
        let connection = self.connection.lock().map_err(settings_database_error)?;
        connection
            .execute(
                r"
                INSERT INTO workspace_settings(setting_key, value_json)
                VALUES (?1, ?2)
                ON CONFLICT(setting_key) DO UPDATE SET value_json = excluded.value_json
                ",
                params![WORKSPACE_SETTINGS_KEY, json],
            )
            .map(|_| ())
            .map_err(settings_sql_error)
    }

    async fn reset_workspace_settings(&self) -> SettingsResult<()> {
        let connection = self.connection.lock().map_err(settings_database_error)?;
        connection
            .execute(
                "DELETE FROM workspace_settings WHERE setting_key = ?1",
                params![WORKSPACE_SETTINGS_KEY],
            )
            .map(|_| ())
            .map_err(settings_sql_error)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WorkspaceSettingsDto {
    schema_version: u32,
    generation: GenerationDefaultsDto,
    image_variants: ImageVariantSettingsDto,
}

impl WorkspaceSettingsDto {
    fn from_domain(value: &WorkspaceSettings) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            generation: GenerationDefaultsDto::from_domain(&value.generation),
            image_variants: ImageVariantSettingsDto::from_domain(value.image_variants),
        }
    }

    fn into_domain(self) -> SettingsResult<WorkspaceSettings> {
        ensure_schema(self.schema_version)?;
        Ok(WorkspaceSettings {
            generation: self.generation.into_domain()?,
            image_variants: self.image_variants.into_domain(),
        })
    }

    fn encode_domain(value: &WorkspaceSettings) -> SettingsResult<String> {
        encode_json(&Self::from_domain(value)).map_err(settings_database_error)
    }

    fn decode_domain(text: &str) -> SettingsResult<WorkspaceSettings> {
        decode_json::<Self>(text)
            .map_err(settings_database_error)?
            .into_domain()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GenerationDefaultsDto {
    model: String,
    width: u32,
    height: u32,
    quality: bool,
    uc_preset: String,
    steps: u32,
    scale: f32,
    sampler: String,
    noise_schedule: String,
    seed: i64,
    n_samples: u32,
    cfg_rescale: f32,
    variety_boost: bool,
    image_format: Option<String>,
    strict_mode: bool,
}

impl GenerationDefaultsDto {
    fn from_domain(value: &GenerationDefaults) -> Self {
        Self {
            model: value.model.as_str().to_owned(),
            width: value.size.width,
            height: value.size.height,
            quality: value.quality,
            uc_preset: uc_preset_as_str(value.uc_preset).to_owned(),
            steps: value.steps,
            scale: value.scale,
            sampler: sampler_as_str(value.sampler).to_owned(),
            noise_schedule: noise_schedule_as_str(value.noise_schedule).to_owned(),
            seed: value.seed,
            n_samples: value.n_samples,
            cfg_rescale: value.cfg_rescale,
            variety_boost: value.variety_boost,
            image_format: value
                .image_format
                .map(image_format_as_str)
                .map(str::to_owned),
            strict_mode: value.strict_mode,
        }
    }

    fn into_domain(self) -> SettingsResult<GenerationDefaults> {
        Ok(GenerationDefaults {
            model: image_model_from_str(&self.model)?,
            size: ImageSize {
                width: self.width,
                height: self.height,
            },
            quality: self.quality,
            uc_preset: uc_preset_from_str(&self.uc_preset)?,
            steps: self.steps,
            scale: self.scale,
            sampler: sampler_from_str(&self.sampler)?,
            noise_schedule: noise_schedule_from_str(&self.noise_schedule)?,
            seed: self.seed,
            n_samples: self.n_samples,
            cfg_rescale: self.cfg_rescale,
            variety_boost: self.variety_boost,
            image_format: self
                .image_format
                .as_deref()
                .map(image_format_from_str)
                .transpose()?,
            strict_mode: self.strict_mode,
        })
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct ImageVariantSettingsDto {
    thumbnail_long_edge: u32,
    preview_long_edge: u32,
}

impl ImageVariantSettingsDto {
    const fn from_domain(value: ImageVariantSettings) -> Self {
        Self {
            thumbnail_long_edge: value.thumbnail_long_edge,
            preview_long_edge: value.preview_long_edge,
        }
    }

    const fn into_domain(self) -> ImageVariantSettings {
        ImageVariantSettings {
            thumbnail_long_edge: self.thumbnail_long_edge,
            preview_long_edge: self.preview_long_edge,
        }
    }
}

fn ensure_schema(version: u32) -> SettingsResult<()> {
    if version == JSON_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(SettingsError::repository(format!(
            "unsupported workspace settings schema version {version}"
        )))
    }
}

fn image_model_from_str(value: &str) -> SettingsResult<ImageModel> {
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

const fn sampler_as_str(value: Sampler) -> &'static str {
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

fn sampler_from_str(value: &str) -> SettingsResult<Sampler> {
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

const fn noise_schedule_as_str(value: NoiseSchedule) -> &'static str {
    match value {
        NoiseSchedule::Karras => "karras",
        NoiseSchedule::Exponential => "exponential",
        NoiseSchedule::Polyexponential => "polyexponential",
    }
}

fn noise_schedule_from_str(value: &str) -> SettingsResult<NoiseSchedule> {
    match value {
        "karras" => Ok(NoiseSchedule::Karras),
        "exponential" => Ok(NoiseSchedule::Exponential),
        "polyexponential" => Ok(NoiseSchedule::Polyexponential),
        _ => Err(decode_error("noise schedule", value)),
    }
}

const fn uc_preset_as_str(value: UcPreset) -> &'static str {
    match value {
        UcPreset::Heavy => "heavy",
        UcPreset::Light => "light",
        UcPreset::FurryFocus => "furry_focus",
        UcPreset::HumanFocus => "human_focus",
        UcPreset::None => "none",
    }
}

fn uc_preset_from_str(value: &str) -> SettingsResult<UcPreset> {
    match value {
        "heavy" => Ok(UcPreset::Heavy),
        "light" => Ok(UcPreset::Light),
        "furry_focus" => Ok(UcPreset::FurryFocus),
        "human_focus" => Ok(UcPreset::HumanFocus),
        "none" => Ok(UcPreset::None),
        _ => Err(decode_error("uc preset", value)),
    }
}

const fn image_format_as_str(value: ImageFormat) -> &'static str {
    match value {
        ImageFormat::Png => "png",
        ImageFormat::Webp => "webp",
    }
}

fn image_format_from_str(value: &str) -> SettingsResult<ImageFormat> {
    match value {
        "png" => Ok(ImageFormat::Png),
        "webp" => Ok(ImageFormat::Webp),
        _ => Err(decode_error("image format", value)),
    }
}

fn decode_error(kind: &str, value: &str) -> SettingsError {
    SettingsError::repository(format!("unknown {kind} `{value}`"))
}

fn settings_database_error(error: DatabaseError) -> SettingsError {
    let message = error.to_string();
    drop(error);
    SettingsError::repository(message)
}

fn settings_sql_error(error: rusqlite::Error) -> SettingsError {
    let message = error.to_string();
    drop(error);
    SettingsError::repository(message)
}
