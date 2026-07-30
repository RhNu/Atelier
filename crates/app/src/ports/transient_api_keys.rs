use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use atelier_secrets::{ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, SecretsError, SecretsResult};

#[derive(Clone, Default)]
pub struct TransientApiKeyRegistryStore {
    records: Arc<Mutex<BTreeMap<ApiKeyId, ApiKeyRecord>>>,
}

#[async_trait]
impl ApiKeyRegistryStore for TransientApiKeyRegistryStore {
    async fn insert_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        let mut records = self.lock_records()?;
        if records.contains_key(&record.id) {
            return Err(SecretsError::validation("api key id already exists"));
        }
        if record.is_active {
            clear_active_api_keys(&mut records);
        }
        records.insert(record.id.clone(), record);
        drop(records);
        Ok(())
    }

    async fn save_api_key_record(&self, record: ApiKeyRecord) -> SecretsResult<()> {
        let mut records = self.lock_records()?;
        if record.is_active {
            clear_active_api_keys(&mut records);
        }
        records.insert(record.id.clone(), record);
        drop(records);
        Ok(())
    }

    async fn get_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<Option<ApiKeyRecord>> {
        Ok(self.lock_records()?.get(id).cloned())
    }

    async fn list_api_key_records(&self) -> SecretsResult<Vec<ApiKeyRecord>> {
        Ok(self.lock_records()?.values().cloned().collect())
    }

    async fn delete_api_key_record(&self, id: &ApiKeyId) -> SecretsResult<bool> {
        Ok(self.lock_records()?.remove(id).is_some())
    }

    async fn set_active_api_key(&self, id: &ApiKeyId) -> SecretsResult<()> {
        let mut records = self.lock_records()?;
        if !records.contains_key(id) {
            return Err(SecretsError::metadata_store("api key does not exist"));
        }
        for record in records.values_mut() {
            record.is_active = &record.id == id;
        }
        drop(records);
        Ok(())
    }

    async fn get_active_api_key_record(&self) -> SecretsResult<Option<ApiKeyRecord>> {
        Ok(self
            .lock_records()?
            .values()
            .find(|record| record.is_active)
            .cloned())
    }
}

impl TransientApiKeyRegistryStore {
    fn lock_records(
        &self,
    ) -> SecretsResult<std::sync::MutexGuard<'_, BTreeMap<ApiKeyId, ApiKeyRecord>>> {
        self.records
            .lock()
            .map_err(|_| SecretsError::metadata_store("API key registry state is unavailable"))
    }
}

fn clear_active_api_keys(records: &mut BTreeMap<ApiKeyId, ApiKeyRecord>) {
    for record in records.values_mut() {
        record.is_active = false;
    }
}
