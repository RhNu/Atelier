use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use futures_executor::block_on;
use nai_atelier_adapter_keyring::{
    KeyringBackend, KeyringBackendError, KeyringSecretStore, SERVICE_NAME,
};
use nai_atelier_secrets::{SecretRecordId, SecretStore, SecretValue, SecretsErrorKind};

#[test]
fn keyring_secret_store_round_trips_through_configured_service_and_account() {
    block_on(async {
        let backend = MemoryKeyringBackend::default();
        let store = KeyringSecretStore::with_backend(backend.clone());
        let id = SecretRecordId::new("novelai-api-key:main");

        store
            .write_secret(&id, SecretValue::new("nai-secret"))
            .await
            .unwrap();
        let secret = store.read_secret(&id).await.unwrap();

        assert_eq!(secret.expose_secret(), "nai-secret");
        assert_eq!(
            backend.last_write().unwrap(),
            (SERVICE_NAME.to_owned(), "novelai-api-key:main".to_owned())
        );
        assert!(store.delete_secret(&id).await.unwrap());
        let error = store.read_secret(&id).await.unwrap_err();
        assert_eq!(error.kind, SecretsErrorKind::MissingSecret);
    });
}

#[test]
fn keyring_secret_store_maps_backend_failures_to_secret_store_errors() {
    block_on(async {
        let backend = MemoryKeyringBackend::failing();
        let store = KeyringSecretStore::with_backend(backend);

        let error = store
            .write_secret(
                &SecretRecordId::new("novelai-api-key:main"),
                SecretValue::new("nai-secret"),
            )
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::SecretStore);
    });
}

#[test]
#[ignore = "touches the OS credential store"]
fn native_keyring_smoke_round_trips_secret() {
    block_on(async {
        let store = KeyringSecretStore::native().unwrap();
        let id = SecretRecordId::new("nai-atelier-test-secret");

        store
            .write_secret(&id, SecretValue::new("native-secret"))
            .await
            .unwrap();
        assert_eq!(
            store.read_secret(&id).await.unwrap().expose_secret(),
            "native-secret"
        );
        assert!(store.delete_secret(&id).await.unwrap());
    });
}

#[derive(Clone, Default)]
struct MemoryKeyringBackend {
    state: Arc<Mutex<MemoryKeyringState>>,
}

#[derive(Default)]
struct MemoryKeyringState {
    entries: BTreeMap<(String, String), String>,
    last_write: Option<(String, String)>,
    fail: bool,
}

impl MemoryKeyringBackend {
    fn failing() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryKeyringState {
                fail: true,
                ..MemoryKeyringState::default()
            })),
        }
    }

    fn last_write(&self) -> Option<(String, String)> {
        self.state.lock().unwrap().last_write.clone()
    }
}

impl KeyringBackend for MemoryKeyringBackend {
    fn write_password(
        &self,
        service: &str,
        account: &str,
        password: &str,
    ) -> Result<(), KeyringBackendError> {
        let mut state = self.state.lock().unwrap();
        if state.fail {
            return Err(KeyringBackendError::Store("write failed".to_owned()));
        }
        state.last_write = Some((service.to_owned(), account.to_owned()));
        state.entries.insert(
            (service.to_owned(), account.to_owned()),
            password.to_owned(),
        );
        drop(state);
        Ok(())
    }

    fn read_password(&self, service: &str, account: &str) -> Result<String, KeyringBackendError> {
        let state = self.state.lock().unwrap();
        state
            .entries
            .get(&(service.to_owned(), account.to_owned()))
            .cloned()
            .ok_or(KeyringBackendError::NoEntry)
    }

    fn delete_password(&self, service: &str, account: &str) -> Result<bool, KeyringBackendError> {
        let removed = self
            .state
            .lock()
            .unwrap()
            .entries
            .remove(&(service.to_owned(), account.to_owned()))
            .is_some();
        Ok(removed)
    }
}
