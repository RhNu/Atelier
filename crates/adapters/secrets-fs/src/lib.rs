//! Filesystem persistence for application-level secret metadata.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use atelier_secrets::{
    ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, SecretRecordId, SecretsError, SecretsResult,
};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

const JSON_FORMAT: &str = "atelier-api-key-registry";
const JSON_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct FileSystemApiKeyRegistryStore {
    path: PathBuf,
    io_lock: Arc<Mutex<()>>,
}

impl FileSystemApiKeyRegistryStore {
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

    fn with_records<T>(
        &self,
        operation: impl FnOnce(&mut BTreeMap<ApiKeyId, ApiKeyRecord>) -> SecretsResult<(T, bool)>,
    ) -> SecretsResult<T> {
        let _guard = self
            .io_lock
            .lock()
            .map_err(|_| metadata_error("API key registry I/O lock is unavailable"))?;
        let mut records = self.load()?;
        let (result, changed) = operation(&mut records)?;
        if changed {
            self.save(&records)?;
        }
        Ok(result)
    }

    fn load(&self) -> SecretsResult<BTreeMap<ApiKeyId, ApiKeyRecord>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(error) => return Err(path_error("read", &self.path, error)),
        };
        let stored: StoredRegistry =
            serde_json::from_str(&text).map_err(|error| metadata_error(error.to_string()))?;
        stored.into_domain()
    }

    fn save(&self, records: &BTreeMap<ApiKeyId, ApiKeyRecord>) -> SecretsResult<()> {
        let stored = StoredRegistry::from_domain(records);
        let text = serde_json::to_string_pretty(&stored)
            .map_err(|error| metadata_error(error.to_string()))?;
        write_registry_file(&self.path, &text)
    }
}

#[async_trait]
impl ApiKeyRegistryStore for FileSystemApiKeyRegistryStore {
    async fn insert_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        self.with_records(|records| {
            if records.contains_key(&record.id) {
                return Err(SecretsError::validation("api key id already exists"));
            }
            if record.is_active {
                clear_active(records);
            }
            records.insert(record.id.clone(), record);
            Ok(((), true))
        })
    }

    async fn save_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        self.with_records(|records| {
            if record.is_active {
                clear_active(records);
            }
            records.insert(record.id.clone(), record);
            Ok(((), true))
        })
    }

    async fn get_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<Option<ApiKeyRecord>> {
        self.with_records(|records| Ok((records.get(id).cloned(), false)))
    }

    async fn list_api_key_records(&self) -> SecretsResult<Vec<ApiKeyRecord>> {
        self.with_records(|records| Ok((records.values().cloned().collect(), false)))
    }

    async fn delete_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<bool> {
        self.with_records(|records| {
            let deleted = records.remove(id).is_some();
            Ok((deleted, deleted))
        })
    }

    async fn set_active_api_key(&self, id: &ApiKeyId) -> SecretsResult<()> {
        self.with_records(|records| {
            if !records.contains_key(id) {
                return Err(metadata_error("api key does not exist"));
            }
            for record in records.values_mut() {
                record.is_active = &record.id == id;
            }
            Ok(((), true))
        })
    }

    async fn get_active_api_key_record(&self) -> SecretsResult<Option<ApiKeyRecord>> {
        self.with_records(|records| {
            Ok((
                records.values().find(|record| record.is_active).cloned(),
                false,
            ))
        })
    }
}

fn clear_active(records: &mut BTreeMap<ApiKeyId, ApiKeyRecord>) {
    for record in records.values_mut() {
        record.is_active = false;
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredRegistry {
    format: String,
    schema_version: u32,
    records: Vec<StoredApiKeyRecord>,
}

impl StoredRegistry {
    fn from_domain(records: &BTreeMap<ApiKeyId, ApiKeyRecord>) -> Self {
        Self {
            format: JSON_FORMAT.to_owned(),
            schema_version: JSON_SCHEMA_VERSION,
            records: records
                .values()
                .map(StoredApiKeyRecord::from_domain)
                .collect(),
        }
    }

    fn into_domain(self) -> SecretsResult<BTreeMap<ApiKeyId, ApiKeyRecord>> {
        if self.format != JSON_FORMAT || self.schema_version != JSON_SCHEMA_VERSION {
            return Err(metadata_error(format!(
                "unsupported API key registry schema `{}` version {}; expected `{JSON_FORMAT}` \
                 version {JSON_SCHEMA_VERSION}",
                self.format, self.schema_version
            )));
        }
        let mut records = BTreeMap::new();
        let mut active_count = 0;
        for stored in self.records {
            let record = stored.into_domain();
            active_count += usize::from(record.is_active);
            if records.insert(record.id.clone(), record).is_some() {
                return Err(metadata_error("API key registry contains duplicate ids"));
            }
        }
        if active_count > 1 {
            return Err(metadata_error(
                "API key registry contains more than one active key",
            ));
        }
        Ok(records)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredApiKeyRecord {
    id: String,
    display_name: String,
    secret_record_id: String,
    is_active: bool,
}

impl StoredApiKeyRecord {
    fn from_domain(record: &ApiKeyRecord) -> Self {
        Self {
            id: record.id.as_str().to_owned(),
            display_name: record.display_name.clone(),
            secret_record_id: record.secret_record_id.as_str().to_owned(),
            is_active: record.is_active,
        }
    }

    fn into_domain(self) -> ApiKeyRecord {
        ApiKeyRecord {
            id: ApiKeyId::new(self.id),
            display_name: self.display_name,
            secret_record_id: SecretRecordId::new(self.secret_record_id),
            is_active: self.is_active,
        }
    }
}

fn write_registry_file(path: &Path, text: &str) -> SecretsResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| metadata_error("API key registry path has no parent directory"))?;
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

fn metadata_error(message: impl Into<String>) -> SecretsError {
    SecretsError::metadata_store(message)
}

fn path_error(operation: &str, path: &Path, error: impl std::fmt::Display) -> SecretsError {
    metadata_error(format!(
        "failed to {operation} API key registry at {}: {error}",
        path.display()
    ))
}
