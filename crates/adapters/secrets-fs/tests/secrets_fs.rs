use atelier_adapter_secrets_fs::FileSystemApiKeyRegistryStore;
use atelier_secrets::{
    ApiKeyId, ApiKeyRecord, ApiKeyRegistryStore, SecretRecordId, SecretsErrorKind,
};
use futures_executor::block_on;

fn record(id: &str, active: bool) -> ApiKeyRecord {
    let id = ApiKeyId::new(id);
    ApiKeyRecord {
        display_name: format!("Key {id_value}", id_value = id.as_str()),
        secret_record_id: SecretRecordId::for_api_key(&id),
        id,
        is_active: active,
    }
}

#[test]
fn registry_persists_application_metadata_without_secret_values() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("api-keys.json");
        let store = FileSystemApiKeyRegistryStore::new(&path);
        store
            .insert_api_key_record(record("main", false))
            .await
            .unwrap();
        store
            .set_active_api_key(&ApiKeyId::new("main"))
            .await
            .unwrap();

        let reopened = FileSystemApiKeyRegistryStore::new(&path);
        let records = reopened.list_api_key_records().await.unwrap();
        assert_eq!(records, vec![record("main", true)]);
        let text = std::fs::read_to_string(path).unwrap();
        let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(stored["records"][0].get("secret").is_none());
    });
}

#[test]
fn registry_keeps_one_active_key_and_rejects_duplicates() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let store = FileSystemApiKeyRegistryStore::new(temp.path().join("api-keys.json"));
        store
            .insert_api_key_record(record("first", true))
            .await
            .unwrap();
        store
            .insert_api_key_record(record("second", true))
            .await
            .unwrap();

        assert_eq!(
            store
                .get_active_api_key_record()
                .await
                .unwrap()
                .unwrap()
                .id
                .as_str(),
            "second"
        );
        let error = store
            .insert_api_key_record(record("second", false))
            .await
            .unwrap_err();
        assert_eq!(error.kind, SecretsErrorKind::Validation);
    });
}
