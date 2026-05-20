mod support;

use futures_executor::block_on;
use nai_atelier_foundation::{NovelAiError, NovelAiErrorKind};
use nai_atelier_secrets::{
    ApiKeyId, ApiKeyRegistryService, ApiKeyRegistryStore, CreateApiKeyRequest, SecretRecordId,
    SecretValue, SecretsErrorKind, UpdateApiKeyRequest,
};
use support::{FakeProbe, MemoryRegistryStore, MemorySecretStore, request};

#[test]
fn secret_value_debug_is_redacted() {
    let secret = SecretValue::new("nai-secret-token");

    let output = format!("{secret:?}");

    assert!(output.contains("<redacted>"));
    assert!(!output.contains("nai-secret-token"));
}

#[test]
fn registry_creates_lists_sets_active_and_resolves_active_secret() {
    block_on(async {
        let metadata = MemoryRegistryStore::default();
        let secrets = MemorySecretStore::default();
        let service =
            ApiKeyRegistryService::new(metadata.clone(), secrets.clone(), FakeProbe::default());

        let record = service
            .create_api_key(CreateApiKeyRequest {
                id: ApiKeyId::new("main"),
                display_name: "Main key".to_owned(),
                secret: SecretValue::new("nai-main-secret"),
            })
            .await
            .unwrap();
        service.set_active_api_key(&record.id).await.unwrap();

        assert_eq!(
            record.secret_record_id,
            SecretRecordId::for_api_key(&ApiKeyId::new("main"))
        );
        assert!(!record.is_active);
        assert_eq!(service.list_api_keys().await.unwrap().len(), 1);
        assert_eq!(
            service
                .resolve_active_secret()
                .await
                .unwrap()
                .expose_secret(),
            "nai-main-secret"
        );
    });
}

#[test]
fn deleting_active_key_does_not_auto_select_another_key() {
    block_on(async {
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            MemorySecretStore::default(),
            FakeProbe::default(),
        );
        service
            .create_api_key(request("first", "First", "secret-1"))
            .await
            .unwrap();
        service
            .create_api_key(request("second", "Second", "secret-2"))
            .await
            .unwrap();
        service
            .set_active_api_key(&ApiKeyId::new("first"))
            .await
            .unwrap();

        assert!(
            service
                .delete_api_key(&ApiKeyId::new("first"))
                .await
                .unwrap()
        );

        let error = service.resolve_active_secret().await.unwrap_err();
        assert_eq!(error.kind, SecretsErrorKind::MissingActiveKey);
        assert_eq!(service.list_api_keys().await.unwrap().len(), 1);
    });
}

#[test]
fn validation_rejects_empty_id_display_name_and_secret() {
    block_on(async {
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            MemorySecretStore::default(),
            FakeProbe::default(),
        );

        for request in [
            request("", "Main", "secret"),
            request("main", "", "secret"),
            request("main", "Main", ""),
        ] {
            let error = service.create_api_key(request).await.unwrap_err();
            assert_eq!(error.kind, SecretsErrorKind::Validation);
        }
    });
}

#[test]
fn update_can_change_display_name_and_secret_without_network_probe() {
    block_on(async {
        let probe = FakeProbe::default();
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            MemorySecretStore::default(),
            probe.clone(),
        );
        service
            .create_api_key(request("main", "Main", "old-secret"))
            .await
            .unwrap();

        let updated = service
            .update_api_key(UpdateApiKeyRequest {
                id: ApiKeyId::new("main"),
                display_name: Some("Renamed".to_owned()),
                secret: Some(SecretValue::new("new-secret")),
            })
            .await
            .unwrap();

        assert_eq!(updated.display_name, "Renamed");
        assert_eq!(
            service
                .resolve_secret_for_key(&ApiKeyId::new("main"))
                .await
                .unwrap()
                .expose_secret(),
            "new-secret"
        );
        assert_eq!(probe.call_count(), 0);
    });
}

#[test]
fn create_rejects_duplicate_id_without_overwriting_existing_secret() {
    block_on(async {
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            MemorySecretStore::default(),
            FakeProbe::default(),
        );
        service
            .create_api_key(request("main", "Main", "old-secret"))
            .await
            .unwrap();
        service
            .set_active_api_key(&ApiKeyId::new("main"))
            .await
            .unwrap();

        let error = service
            .create_api_key(request("main", "Duplicate", "new-secret"))
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::Validation);
        assert_eq!(
            service
                .resolve_active_secret()
                .await
                .unwrap()
                .expose_secret(),
            "old-secret"
        );
    });
}

#[test]
fn create_removes_secret_when_metadata_save_fails() {
    block_on(async {
        let metadata = MemoryRegistryStore::failing_save();
        let secrets = MemorySecretStore::default();
        let service = ApiKeyRegistryService::new(metadata, secrets.clone(), FakeProbe::default());

        let error = service
            .create_api_key(request("main", "Main", "secret"))
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::MetadataStore);
        assert!(!secrets.contains(&SecretRecordId::for_api_key(&ApiKeyId::new("main"))));
    });
}

#[test]
fn create_removes_metadata_when_secret_write_fails() {
    block_on(async {
        let metadata = MemoryRegistryStore::default();
        let service = ApiKeyRegistryService::new(
            metadata.clone(),
            MemorySecretStore::failing_write(),
            FakeProbe::default(),
        );

        let error = service
            .create_api_key(request("main", "Main", "secret"))
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::SecretStore);
        assert!(metadata.list_api_key_records().await.unwrap().is_empty());
    });
}

#[test]
fn update_restores_previous_secret_when_metadata_save_fails() {
    block_on(async {
        let metadata = MemoryRegistryStore::default();
        let service = ApiKeyRegistryService::new(
            metadata.clone(),
            MemorySecretStore::default(),
            FakeProbe::default(),
        );
        service
            .create_api_key(request("main", "Main", "old-secret"))
            .await
            .unwrap();
        metadata.set_fail_save(true);

        let error = service
            .update_api_key(UpdateApiKeyRequest {
                id: ApiKeyId::new("main"),
                display_name: Some("Renamed".to_owned()),
                secret: Some(SecretValue::new("new-secret")),
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::MetadataStore);
        assert_eq!(
            service
                .resolve_secret_for_key(&ApiKeyId::new("main"))
                .await
                .unwrap()
                .expose_secret(),
            "old-secret"
        );
    });
}

#[test]
fn update_restores_metadata_when_secret_write_fails() {
    block_on(async {
        let secrets = MemorySecretStore::default();
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            secrets.clone(),
            FakeProbe::default(),
        );
        service
            .create_api_key(request("main", "Main", "old-secret"))
            .await
            .unwrap();
        secrets.set_fail_write(true);

        let error = service
            .update_api_key(UpdateApiKeyRequest {
                id: ApiKeyId::new("main"),
                display_name: Some("Renamed".to_owned()),
                secret: Some(SecretValue::new("new-secret")),
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::SecretStore);
        assert_eq!(
            service
                .list_api_keys()
                .await
                .unwrap()
                .first()
                .unwrap()
                .display_name,
            "Main"
        );
        assert_eq!(
            service
                .resolve_secret_for_key(&ApiKeyId::new("main"))
                .await
                .unwrap()
                .expose_secret(),
            "old-secret"
        );
    });
}

#[test]
fn update_restores_previous_secret_when_secret_write_partially_fails() {
    block_on(async {
        let secrets = MemorySecretStore::default();
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            secrets.clone(),
            FakeProbe::default(),
        );
        service
            .create_api_key(request("main", "Main", "old-secret"))
            .await
            .unwrap();
        secrets.fail_next_write_after_store();

        let error = service
            .update_api_key(UpdateApiKeyRequest {
                id: ApiKeyId::new("main"),
                display_name: None,
                secret: Some(SecretValue::new("new-secret")),
            })
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::SecretStore);
        assert_eq!(
            service
                .resolve_secret_for_key(&ApiKeyId::new("main"))
                .await
                .unwrap()
                .expose_secret(),
            "old-secret"
        );
    });
}

#[test]
fn delete_restores_secret_when_metadata_delete_fails() {
    block_on(async {
        let metadata = MemoryRegistryStore::default();
        let service = ApiKeyRegistryService::new(
            metadata.clone(),
            MemorySecretStore::default(),
            FakeProbe::default(),
        );
        service
            .create_api_key(request("main", "Main", "secret"))
            .await
            .unwrap();
        metadata.set_fail_delete(true);

        let error = service
            .delete_api_key(&ApiKeyId::new("main"))
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::MetadataStore);
        assert_eq!(
            service
                .resolve_secret_for_key(&ApiKeyId::new("main"))
                .await
                .unwrap()
                .expose_secret(),
            "secret"
        );
    });
}

#[test]
fn delete_keeps_metadata_reachable_when_secret_delete_fails() {
    block_on(async {
        let metadata = MemoryRegistryStore::default();
        let secrets = MemorySecretStore::failing_delete();
        let service =
            ApiKeyRegistryService::new(metadata.clone(), secrets.clone(), FakeProbe::default());
        service
            .create_api_key(request("main", "Main", "secret"))
            .await
            .unwrap();

        let error = service
            .delete_api_key(&ApiKeyId::new("main"))
            .await
            .unwrap_err();

        assert_eq!(error.kind, SecretsErrorKind::SecretStore);
        assert!(
            metadata
                .get_api_key_record(&ApiKeyId::new("main"))
                .await
                .unwrap()
                .is_some()
        );
        assert!(secrets.contains(&SecretRecordId::for_api_key(&ApiKeyId::new("main"))));
    });
}

#[test]
fn probe_key_uses_explicit_probe_path() {
    block_on(async {
        let probe = FakeProbe::default();
        let service = ApiKeyRegistryService::new(
            MemoryRegistryStore::default(),
            MemorySecretStore::default(),
            probe.clone(),
        );
        service
            .create_api_key(request("main", "Main", "secret-for-probe"))
            .await
            .unwrap();

        let summary = service.probe_key(&ApiKeyId::new("main")).await.unwrap();

        assert_eq!(summary.anlas_balance, 42);
        assert_eq!(probe.secrets(), vec!["secret-for-probe".to_owned()]);
    });
}

#[test]
fn fake_probe_can_surface_novelai_errors() {
    let error = NovelAiError::new(NovelAiErrorKind::Authentication, "bad key");
    assert_eq!(error.kind, NovelAiErrorKind::Authentication);
}
