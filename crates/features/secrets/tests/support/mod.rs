use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_secrets::{
    ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, CreateApiKeyRequest, SecretRecordId, SecretStore,
    SecretValue, SecretsError, SecretsResult, SubscriptionClientError, SubscriptionProbeClient,
    SubscriptionSummary,
};

pub fn request(id: &str, display_name: &str, secret: &str) -> CreateApiKeyRequest {
    CreateApiKeyRequest {
        id: ApiKeyId::new(id),
        display_name: display_name.to_owned(),
        secret: SecretValue::new(secret),
    }
}

#[derive(Clone, Default)]
pub struct MemoryRegistryStore {
    state: Arc<Mutex<MemoryRegistryState>>,
}

#[derive(Default)]
struct MemoryRegistryState {
    records: BTreeMap<ApiKeyId, ApiKeyRecord>,
    fail_save: bool,
    fail_delete: bool,
}

impl MemoryRegistryStore {
    pub fn failing_save() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryRegistryState {
                fail_save: true,
                ..MemoryRegistryState::default()
            })),
        }
    }

    pub fn set_fail_save(&self, fail_save: bool) {
        self.state.lock().unwrap().fail_save = fail_save;
    }

    pub fn set_fail_delete(&self, fail_delete: bool) {
        self.state.lock().unwrap().fail_delete = fail_delete;
    }
}

#[async_trait]
impl ApiKeyRegistryStore for MemoryRegistryStore {
    async fn insert_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        let mut state = self.state.lock().unwrap();
        if state.fail_save {
            return Err(SecretsError::metadata_store("save failed"));
        }
        if state.records.contains_key(&record.id) {
            return Err(SecretsError::validation("api key id already exists"));
        }
        if record.is_active {
            for existing in state.records.values_mut() {
                existing.is_active = false;
            }
        }
        state.records.insert(record.id.clone(), record);
        drop(state);
        Ok(())
    }

    async fn save_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        let mut state = self.state.lock().unwrap();
        if state.fail_save {
            return Err(SecretsError::metadata_store("save failed"));
        }
        if record.is_active {
            for existing in state.records.values_mut() {
                existing.is_active = false;
            }
        }
        state.records.insert(record.id.clone(), record);
        drop(state);
        Ok(())
    }

    async fn get_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<Option<ApiKeyRecord>> {
        Ok(self.state.lock().unwrap().records.get(id).cloned())
    }

    async fn list_api_key_records(&self) -> SecretsResult<Vec<ApiKeyRecord>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .records
            .values()
            .cloned()
            .collect())
    }

    async fn delete_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<bool> {
        if self.state.lock().unwrap().fail_delete {
            return Err(SecretsError::metadata_store("delete failed"));
        }
        Ok(self.state.lock().unwrap().records.remove(id).is_some())
    }

    async fn set_active_api_key(&self, id: &ApiKeyId) -> SecretsResult<()> {
        let mut state = self.state.lock().unwrap();
        if !state.records.contains_key(id) {
            return Err(SecretsError::metadata_store("api key does not exist"));
        }
        for record in state.records.values_mut() {
            record.is_active = &record.id == id;
        }
        drop(state);
        Ok(())
    }

    async fn get_active_api_key_record(&self) -> SecretsResult<Option<ApiKeyRecord>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .records
            .values()
            .find(|record| record.is_active)
            .cloned())
    }
}

#[derive(Clone, Default)]
pub struct MemorySecretStore {
    secrets: Arc<Mutex<BTreeMap<SecretRecordId, SecretValue>>>,
    fail_write: Arc<Mutex<bool>>,
    fail_next_write_after_store: Arc<Mutex<bool>>,
    fail_delete: Arc<Mutex<bool>>,
}

impl MemorySecretStore {
    pub fn failing_write() -> Self {
        Self {
            fail_write: Arc::new(Mutex::new(true)),
            ..Self::default()
        }
    }

    pub fn failing_delete() -> Self {
        Self {
            fail_delete: Arc::new(Mutex::new(true)),
            ..Self::default()
        }
    }

    pub fn set_fail_write(&self, fail_write: bool) {
        *self.fail_write.lock().unwrap() = fail_write;
    }

    pub fn fail_next_write_after_store(&self) {
        *self.fail_next_write_after_store.lock().unwrap() = true;
    }

    pub fn contains(&self, id: &SecretRecordId) -> bool {
        self.secrets.lock().unwrap().contains_key(id)
    }
}

#[async_trait]
impl SecretStore for MemorySecretStore {
    async fn write_secret(&self, id: &SecretRecordId, secret: SecretValue) -> SecretsResult<()> {
        if *self.fail_write.lock().unwrap() {
            return Err(SecretsError::secret_store("write failed"));
        }
        if *self.fail_next_write_after_store.lock().unwrap() {
            *self.fail_next_write_after_store.lock().unwrap() = false;
            self.secrets.lock().unwrap().insert(id.clone(), secret);
            return Err(SecretsError::secret_store("write failed after store"));
        }
        self.secrets.lock().unwrap().insert(id.clone(), secret);
        Ok(())
    }

    async fn read_secret(&self, id: &SecretRecordId) -> SecretsResult<SecretValue> {
        self.secrets
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| SecretsError::missing_secret(id.as_str()))
    }

    async fn delete_secret(&self, id: &SecretRecordId) -> SecretsResult<bool> {
        if *self.fail_delete.lock().unwrap() {
            return Err(SecretsError::secret_store("delete failed"));
        }
        Ok(self.secrets.lock().unwrap().remove(id).is_some())
    }
}

#[derive(Clone, Default)]
pub struct FakeProbe {
    calls: Arc<Mutex<Vec<String>>>,
}

impl FakeProbe {
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    pub fn secrets(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl SubscriptionProbeClient for FakeProbe {
    async fn probe_subscription(
        &self,
        secret: SecretValue,
    ) -> Result<SubscriptionSummary, SubscriptionClientError> {
        self.calls
            .lock()
            .unwrap()
            .push(secret.expose_secret().to_owned());
        Ok(SubscriptionSummary {
            anlas_balance: 42,
            is_opus: true,
            tier: 3,
            tier_name: "opus".to_owned(),
            expires_at_ms: Some(1_700_000_000_000),
            v5_usage: None,
        })
    }
}
