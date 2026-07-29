//! Filesystem persistence for user-level Atelier settings.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use atelier_settings::{
    FrontendLanguage, GlobalFrontendSettings, GlobalGallerySettings, GlobalSettings,
    GlobalSettingsRepository, SettingsError, SettingsResult,
};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

const JSON_FORMAT: &str = "atelier-global-settings";
const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub struct FileSystemGlobalSettingsRepository {
    path: PathBuf,
    io_lock: Mutex<()>,
}

impl FileSystemGlobalSettingsRepository {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            io_lock: Mutex::new(()),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load(&self) -> SettingsResult<GlobalSettings> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| repository_error("global settings I/O lock is unavailable"))?;
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(GlobalSettings::default());
            }
            Err(error) => return Err(path_error("read", &self.path, error)),
        };
        match serde_json::from_str::<StoredGlobalSettings>(&text)
            .map_err(|error| error.to_string())
            .and_then(StoredGlobalSettings::into_domain)
        {
            Ok(settings) => Ok(settings),
            Err(error) => {
                let invalid_path = invalid_settings_path(&self.path);
                fs::rename(&self.path, &invalid_path)
                    .map_err(|source| path_error("quarantine", &self.path, source))?;
                log::warn!(
                    "invalid global settings moved to {}: {error}",
                    invalid_path.display()
                );
                Ok(GlobalSettings::default())
            }
        }
    }

    fn save(&self, settings: &GlobalSettings) -> SettingsResult<()> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| repository_error("global settings I/O lock is unavailable"))?;
        let parent = self
            .path
            .parent()
            .ok_or_else(|| repository_error("global settings path has no parent directory"))?;
        fs::create_dir_all(parent).map_err(|error| path_error("create", parent, error))?;
        let text = serde_json::to_string_pretty(&StoredGlobalSettings::from_domain(settings))
            .map_err(|error| repository_error(error.to_string()))?;
        let mut temporary = NamedTempFile::new_in(parent)
            .map_err(|error| path_error("create temporary file in", parent, error))?;
        temporary
            .write_all(text.as_bytes())
            .and_then(|()| temporary.write_all(b"\n"))
            .and_then(|()| temporary.as_file_mut().sync_all())
            .map_err(|error| path_error("write", temporary.path(), error))?;
        temporary
            .persist(&self.path)
            .map_err(|error| path_error("replace", &self.path, error.error))?;
        Ok(())
    }
}

#[async_trait]
impl GlobalSettingsRepository for FileSystemGlobalSettingsRepository {
    async fn get_global_settings(&self) -> SettingsResult<GlobalSettings> {
        self.load()
    }

    async fn save_global_settings(&self, settings: GlobalSettings) -> SettingsResult<()> {
        self.save(&settings)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredGlobalSettings {
    format: String,
    schema_version: u32,
    last_workspace: Option<PathBuf>,
    frontend: StoredGlobalFrontendSettings,
}

impl StoredGlobalSettings {
    fn from_domain(settings: &GlobalSettings) -> Self {
        Self {
            format: JSON_FORMAT.to_owned(),
            schema_version: JSON_SCHEMA_VERSION,
            last_workspace: settings.last_workspace.clone(),
            frontend: StoredGlobalFrontendSettings::from_domain(settings.frontend),
        }
    }

    fn into_domain(self) -> Result<GlobalSettings, String> {
        if self.format != JSON_FORMAT || self.schema_version != JSON_SCHEMA_VERSION {
            return Err(format!(
                "unsupported global settings schema `{}` version {}; expected `{JSON_FORMAT}` \
                 version {JSON_SCHEMA_VERSION}",
                self.format, self.schema_version
            ));
        }
        Ok(GlobalSettings {
            last_workspace: self.last_workspace,
            frontend: self.frontend.into_domain(),
        })
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
struct StoredGlobalFrontendSettings {
    language: StoredFrontendLanguage,
    developer_mode: bool,
    gallery: StoredGlobalGallerySettings,
}

impl StoredGlobalFrontendSettings {
    const fn from_domain(settings: GlobalFrontendSettings) -> Self {
        Self {
            language: StoredFrontendLanguage::from_domain(settings.language),
            developer_mode: settings.developer_mode,
            gallery: StoredGlobalGallerySettings::from_domain(settings.gallery),
        }
    }

    const fn into_domain(self) -> GlobalFrontendSettings {
        GlobalFrontendSettings {
            language: self.language.into_domain(),
            developer_mode: self.developer_mode,
            gallery: self.gallery.into_domain(),
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
enum StoredFrontendLanguage {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "zh-CN")]
    SimplifiedChinese,
}

impl StoredFrontendLanguage {
    const fn from_domain(language: FrontendLanguage) -> Self {
        match language {
            FrontendLanguage::System => Self::System,
            FrontendLanguage::English => Self::English,
            FrontendLanguage::SimplifiedChinese => Self::SimplifiedChinese,
        }
    }

    const fn into_domain(self) -> FrontendLanguage {
        match self {
            Self::System => FrontendLanguage::System,
            Self::English => FrontendLanguage::English,
            Self::SimplifiedChinese => FrontendLanguage::SimplifiedChinese,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
struct StoredGlobalGallerySettings {
    blur_sensitive_images: bool,
}

impl StoredGlobalGallerySettings {
    const fn from_domain(settings: GlobalGallerySettings) -> Self {
        Self {
            blur_sensitive_images: settings.blur_sensitive_images,
        }
    }

    const fn into_domain(self) -> GlobalGallerySettings {
        GlobalGallerySettings {
            blur_sensitive_images: self.blur_sensitive_images,
        }
    }
}

fn invalid_settings_path(path: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("global-settings.json");
    path.with_file_name(format!("{file_name}.invalid-{timestamp}"))
}

fn repository_error(message: impl Into<String>) -> SettingsError {
    SettingsError::repository(message)
}

fn path_error(operation: &str, path: &Path, error: impl std::fmt::Display) -> SettingsError {
    repository_error(format!(
        "failed to {operation} global settings at {}: {error}",
        path.display()
    ))
}
