//! Filesystem persistence for application-global Agent connections and models.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use atelier_agent::{
    AgentAuth, AgentConnection, AgentConnectionId, AgentError, AgentModel, AgentModelId,
    AgentProbeStatus, AgentRegistry, AgentRegistryRepository, AgentResult,
};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

const JSON_FORMAT: &str = "atelier-agent-registry";
const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct FileSystemAgentRegistryRepository {
    path: PathBuf,
    io_lock: Arc<Mutex<()>>,
}

impl FileSystemAgentRegistryRepository {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            io_lock: Arc::new(Mutex::new(())),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load(&self) -> AgentResult<AgentRegistry> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AgentRegistry::default());
            }
            Err(error) => return Err(path_error("read", &self.path, error)),
        };
        let stored: StoredRegistry =
            serde_json::from_str(&text).map_err(|error| registry_error(error.to_string()))?;
        stored.into_domain()
    }

    fn save(&self, registry: &AgentRegistry) -> AgentResult<()> {
        registry.validate()?;
        let text = serde_json::to_string_pretty(&StoredRegistry::from_domain(registry))
            .map_err(|error| registry_error(error.to_string()))?;
        write_registry_file(&self.path, &text)
    }
}

#[async_trait]
impl AgentRegistryRepository for FileSystemAgentRegistryRepository {
    async fn load_registry(&self) -> AgentResult<AgentRegistry> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| registry_error("agent registry I/O lock is unavailable"))?;
        self.load()
    }

    async fn save_registry(&self, registry: AgentRegistry) -> AgentResult<()> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| registry_error("agent registry I/O lock is unavailable"))?;
        self.save(&registry)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredRegistry {
    format: String,
    schema_version: u32,
    connections: Vec<StoredConnection>,
    models: Vec<StoredModel>,
}

impl StoredRegistry {
    fn from_domain(registry: &AgentRegistry) -> Self {
        Self {
            format: JSON_FORMAT.to_owned(),
            schema_version: JSON_SCHEMA_VERSION,
            connections: registry
                .connections
                .iter()
                .map(StoredConnection::from_domain)
                .collect(),
            models: registry
                .models
                .iter()
                .map(StoredModel::from_domain)
                .collect(),
        }
    }

    fn into_domain(self) -> AgentResult<AgentRegistry> {
        if self.format != JSON_FORMAT || self.schema_version != JSON_SCHEMA_VERSION {
            return Err(registry_error(format!(
                "unsupported Agent registry schema `{}` version {}; expected `{JSON_FORMAT}` version {JSON_SCHEMA_VERSION}",
                self.format, self.schema_version
            )));
        }
        let registry = AgentRegistry {
            connections: self
                .connections
                .into_iter()
                .map(StoredConnection::into_domain)
                .collect(),
            models: self
                .models
                .into_iter()
                .map(StoredModel::into_domain)
                .collect(),
        };
        registry.validate()?;
        Ok(registry)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredConnection {
    id: String,
    display_name: String,
    base_url: String,
    auth: StoredAuth,
    created_at_ms: u64,
    updated_at_ms: u64,
}

impl StoredConnection {
    fn from_domain(value: &AgentConnection) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            display_name: value.display_name.clone(),
            base_url: value.base_url.clone(),
            auth: StoredAuth::from_domain(&value.auth),
            created_at_ms: value.created_at_ms,
            updated_at_ms: value.updated_at_ms,
        }
    }

    fn into_domain(self) -> AgentConnection {
        AgentConnection {
            id: AgentConnectionId::new(self.id),
            display_name: self.display_name,
            base_url: self.base_url,
            auth: self.auth.into_domain(),
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StoredAuth {
    None,
    Bearer { secret_record_id: String },
}

impl StoredAuth {
    fn from_domain(value: &AgentAuth) -> Self {
        match value {
            AgentAuth::None => Self::None,
            AgentAuth::Bearer { secret_record_id } => Self::Bearer {
                secret_record_id: secret_record_id.clone(),
            },
        }
    }

    fn into_domain(self) -> AgentAuth {
        match self {
            Self::None => AgentAuth::None,
            Self::Bearer { secret_record_id } => AgentAuth::Bearer { secret_record_id },
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredModel {
    id: String,
    connection_id: String,
    wire_model_id: String,
    display_name: String,
    context_window: u32,
    max_output_tokens: u32,
    temperature: f32,
    probe_status: StoredProbeStatus,
    updated_at_ms: u64,
}

impl StoredModel {
    fn from_domain(value: &AgentModel) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            connection_id: value.connection_id.as_str().to_owned(),
            wire_model_id: value.wire_model_id.clone(),
            display_name: value.display_name.clone(),
            context_window: value.context_window,
            max_output_tokens: value.max_output_tokens,
            temperature: value.temperature,
            probe_status: StoredProbeStatus::from_domain(value.probe_status),
            updated_at_ms: value.updated_at_ms,
        }
    }

    fn into_domain(self) -> AgentModel {
        AgentModel {
            id: AgentModelId::new(self.id),
            connection_id: AgentConnectionId::new(self.connection_id),
            wire_model_id: self.wire_model_id,
            display_name: self.display_name,
            context_window: self.context_window,
            max_output_tokens: self.max_output_tokens,
            temperature: self.temperature,
            probe_status: self.probe_status.into_domain(),
            updated_at_ms: self.updated_at_ms,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum StoredProbeStatus {
    Unknown,
    Verified,
    Failed,
}

impl StoredProbeStatus {
    const fn from_domain(value: AgentProbeStatus) -> Self {
        match value {
            AgentProbeStatus::Unknown => Self::Unknown,
            AgentProbeStatus::Verified => Self::Verified,
            AgentProbeStatus::Failed => Self::Failed,
        }
    }

    const fn into_domain(self) -> AgentProbeStatus {
        match self {
            Self::Unknown => AgentProbeStatus::Unknown,
            Self::Verified => AgentProbeStatus::Verified,
            Self::Failed => AgentProbeStatus::Failed,
        }
    }
}

fn write_registry_file(path: &Path, text: &str) -> AgentResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| registry_error("agent registry path has no parent directory"))?;
    fs::create_dir_all(parent).map_err(|error| path_error("create", parent, error))?;
    let mut temporary = NamedTempFile::new_in(parent)
        .map_err(|error| path_error("create temporary file in", parent, error))?;
    temporary
        .write_all(text.as_bytes())
        .and_then(|()| temporary.write_all(b"\n"))
        .and_then(|()| temporary.as_file_mut().sync_all())
        .map_err(|error| path_error("write", temporary.path(), error))?;
    temporary
        .persist(path)
        .map_err(|error| path_error("replace", path, error.error))?;
    Ok(())
}

fn registry_error(message: impl Into<String>) -> AgentError {
    AgentError::repository(message)
}

fn path_error(operation: &str, path: &Path, error: impl std::fmt::Display) -> AgentError {
    registry_error(format!(
        "failed to {operation} Agent registry at {}: {error}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_registry_without_secret_values() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("agent-registry.json");
        let repository = FileSystemAgentRegistryRepository::new(&path);
        let registry = AgentRegistry {
            connections: vec![AgentConnection {
                id: AgentConnectionId::new("local"),
                display_name: "Local".to_owned(),
                base_url: "http://127.0.0.1:1234/v1".to_owned(),
                auth: AgentAuth::Bearer {
                    secret_record_id: "agent-connection:local".to_owned(),
                },
                created_at_ms: 1,
                updated_at_ms: 2,
            }],
            models: vec![AgentModel {
                id: AgentModelId::new("local-model"),
                connection_id: AgentConnectionId::new("local"),
                wire_model_id: "model".to_owned(),
                display_name: "Model".to_owned(),
                context_window: 32_768,
                max_output_tokens: 4_096,
                temperature: 0.3,
                probe_status: AgentProbeStatus::Verified,
                updated_at_ms: 2,
            }],
        };

        futures_executor::block_on(repository.save_registry(registry.clone())).expect("save");
        let loaded = futures_executor::block_on(repository.load_registry()).expect("load");
        assert_eq!(loaded, registry);
        let text = std::fs::read_to_string(path).expect("read");
        assert!(!text.contains("api-key-value"));
    }
}
