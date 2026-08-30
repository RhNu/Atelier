use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use atelier_downloadable_resources::{DownloadableResourceError, DownloadableResourceResult};
use serde::{Deserialize, Serialize};

const STATE_FORMAT: &str = "atelier.downloadable-resource-state";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstalledState {
    format: String,
    schema_version: u32,
    pub active: HashMap<String, String>,
    #[serde(default)]
    pub pending_delete: HashSet<String>,
    #[serde(default)]
    pub legacy_cleanup_complete: bool,
    #[serde(default)]
    pub onboarding_complete: bool,
}

impl Default for InstalledState {
    fn default() -> Self {
        Self {
            format: STATE_FORMAT.to_owned(),
            schema_version: 1,
            active: HashMap::new(),
            pending_delete: HashSet::new(),
            legacy_cleanup_complete: false,
            onboarding_complete: false,
        }
    }
}

impl InstalledState {
    pub fn read(path: &Path) -> DownloadableResourceResult<Self> {
        if !path.is_file() {
            return Ok(Self::default());
        }
        let bytes = fs::read(path).map_err(operation)?;
        let value: Self = serde_json::from_slice(&bytes).map_err(operation)?;
        if value.format != STATE_FORMAT || value.schema_version != 1 {
            return Err(DownloadableResourceError::Operation(
                "unsupported installed resource state".to_owned(),
            ));
        }
        Ok(value)
    }

    pub fn write(&self, path: &Path) -> DownloadableResourceResult<()> {
        write_json_atomic(path, self)
    }
}

pub fn write_json_atomic(path: &Path, value: &impl Serialize) -> DownloadableResourceResult<()> {
    let parent = path.parent().ok_or_else(|| {
        DownloadableResourceError::Operation("state path has no parent".to_owned())
    })?;
    fs::create_dir_all(parent).map_err(operation)?;
    let temporary = temporary_path(path);
    let mut bytes = serde_json::to_vec_pretty(value).map_err(operation)?;
    bytes.push(b'\n');
    fs::write(&temporary, bytes).map_err(operation)?;
    if path.exists() {
        fs::remove_file(path).map_err(operation)?;
    }
    fs::rename(temporary, path).map_err(operation)
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension("json.part")
}

pub fn operation(error: impl std::fmt::Display) -> DownloadableResourceError {
    DownloadableResourceError::Operation(error.to_string())
}
