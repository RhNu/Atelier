//! System keyring adapter for Atelier secrets.

use async_trait::async_trait;
use atelier_secrets::{SecretRecordId, SecretStore, SecretValue, SecretsError, SecretsResult};
use keyring_core::{Entry, Error as KeyringCoreError};

pub const SERVICE_NAME: &str = "atelier";

#[derive(Clone, Debug)]
pub enum KeyringBackendError {
    NoEntry,
    Store(String),
}

impl From<KeyringCoreError> for KeyringBackendError {
    fn from(error: KeyringCoreError) -> Self {
        match error {
            KeyringCoreError::NoEntry => Self::NoEntry,
            other => Self::Store(other.to_string()),
        }
    }
}

pub trait KeyringBackend: Clone + Send + Sync {
    /// Writes a password to a backend entry.
    ///
    /// # Errors
    /// Returns an error when the backend cannot create or update the entry.
    fn write_password(
        &self,
        service: &str,
        account: &str,
        password: &str,
    ) -> Result<(), KeyringBackendError>;

    /// Reads a password from a backend entry.
    ///
    /// # Errors
    /// Returns an error when the entry is missing or cannot be read.
    fn read_password(&self, service: &str, account: &str) -> Result<String, KeyringBackendError>;

    /// Deletes a backend entry.
    ///
    /// # Errors
    /// Returns an error when the backend cannot delete the entry.
    fn delete_password(&self, service: &str, account: &str) -> Result<bool, KeyringBackendError>;
}

#[derive(Clone, Debug)]
pub struct KeyringSecretStore<B = NativeKeyringBackend> {
    backend: B,
}

impl KeyringSecretStore<NativeKeyringBackend> {
    /// Creates a secret store backed by the platform native credential store.
    ///
    /// # Errors
    /// Returns an error when the native keyring store cannot be selected.
    pub fn native() -> SecretsResult<Self> {
        NativeKeyringBackend::ensure_native()?;
        Ok(Self::with_backend(NativeKeyringBackend))
    }
}

impl<B> KeyringSecretStore<B>
where
    B: KeyringBackend,
{
    #[must_use]
    pub const fn with_backend(backend: B) -> Self {
        Self { backend }
    }
}

#[async_trait]
impl<B> SecretStore for KeyringSecretStore<B>
where
    B: KeyringBackend,
{
    async fn write_secret(&self, id: &SecretRecordId, secret: SecretValue) -> SecretsResult<()> {
        self.backend
            .write_password(SERVICE_NAME, id.as_str(), secret.expose_secret())
            .map_err(|error| map_keyring_error(id, error))
    }

    async fn read_secret(&self, id: &SecretRecordId) -> SecretsResult<SecretValue> {
        self.backend
            .read_password(SERVICE_NAME, id.as_str())
            .map(SecretValue::new)
            .map_err(|error| map_keyring_error(id, error))
    }

    async fn delete_secret(&self, id: &SecretRecordId) -> SecretsResult<bool> {
        match self.backend.delete_password(SERVICE_NAME, id.as_str()) {
            Ok(deleted) => Ok(deleted),
            Err(KeyringBackendError::NoEntry) => Ok(false),
            Err(error) => Err(map_keyring_error(id, error)),
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct NativeKeyringBackend;

impl NativeKeyringBackend {
    fn ensure_native() -> SecretsResult<()> {
        keyring::use_native_store(false)
            .map_err(|error| SecretsError::secret_store(error.to_string()))
    }
}

impl KeyringBackend for NativeKeyringBackend {
    fn write_password(
        &self,
        service: &str,
        account: &str,
        password: &str,
    ) -> Result<(), KeyringBackendError> {
        Self::ensure_native().map_err(|error| KeyringBackendError::Store(error.to_string()))?;
        Entry::new(service, account)?
            .set_password(password)
            .map_err(Into::into)
    }

    fn read_password(&self, service: &str, account: &str) -> Result<String, KeyringBackendError> {
        Self::ensure_native().map_err(|error| KeyringBackendError::Store(error.to_string()))?;
        Entry::new(service, account)?
            .get_password()
            .map_err(Into::into)
    }

    fn delete_password(&self, service: &str, account: &str) -> Result<bool, KeyringBackendError> {
        Self::ensure_native().map_err(|error| KeyringBackendError::Store(error.to_string()))?;
        match Entry::new(service, account)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(KeyringCoreError::NoEntry) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

fn map_keyring_error(id: &SecretRecordId, error: KeyringBackendError) -> SecretsError {
    match error {
        KeyringBackendError::NoEntry => SecretsError::missing_secret(id.as_str()),
        KeyringBackendError::Store(message) => SecretsError::secret_store(message),
    }
}
