use super::*;

#[test]
fn migrations_are_idempotent_and_file_backed_database_reopens() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let first = DatabaseConnection::open(&path).unwrap();
        first.run_migrations().unwrap();
        let repository = DatabaseResourceCatalogRepository::new(first.clone());
        let catalog =
            ResourceCatalog::new(repository, MemoryBlobStore::default(), NullVariantBuilder);

        let reference = catalog
            .register_resource(generated_resource("persisted-res", vec![1, 2, 3]))
            .await
            .unwrap();
        drop(catalog);
        drop(first);

        let reopened = DatabaseConnection::open(&path).unwrap();
        reopened.run_migrations().unwrap();
        let repository = DatabaseResourceCatalogRepository::new(reopened);
        let record = repository.get_ready_record(&reference.id).await.unwrap();

        assert_eq!(record.unwrap().metadata.byte_size, Some(3));
    });
}

#[test]
fn api_key_registry_store_round_trips_metadata_without_secret_value() {
    block_on(async {
        let store = DatabaseApiKeyRegistryStore::new(DatabaseConnection::open_memory().unwrap());
        let record = api_key_record("main", "Main key", false);

        store.save_api_key_record(record.clone()).await.unwrap();

        assert_eq!(
            store
                .get_api_key_record(&ApiKeyId::new("main"))
                .await
                .unwrap(),
            Some(record.clone())
        );
        assert_eq!(store.list_api_key_records().await.unwrap(), vec![record]);
    });
}

#[test]
fn migrations_add_api_key_registry_to_existing_v1_database() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch(
                    r"
                    CREATE TABLE schema_migrations (
                        version INTEGER PRIMARY KEY,
                        applied_at_ms INTEGER NOT NULL DEFAULT (
                            CAST(strftime('%s', 'now') AS INTEGER) * 1000
                        )
                    );
                    INSERT INTO schema_migrations(version) VALUES (1);
                    ",
                )
                .unwrap();
        }

        let store = DatabaseApiKeyRegistryStore::new(DatabaseConnection::open(&path).unwrap());
        let record = api_key_record("main", "Main key", false);

        store.save_api_key_record(record.clone()).await.unwrap();

        assert_eq!(
            store
                .get_api_key_record(&ApiKeyId::new("main"))
                .await
                .unwrap(),
            Some(record)
        );
    });
}

#[test]
fn api_key_registry_store_keeps_one_active_key_and_does_not_auto_select_on_delete() {
    block_on(async {
        let store = DatabaseApiKeyRegistryStore::new(DatabaseConnection::open_memory().unwrap());
        store
            .save_api_key_record(api_key_record("first", "First", false))
            .await
            .unwrap();
        store
            .save_api_key_record(api_key_record("second", "Second", false))
            .await
            .unwrap();

        store
            .set_active_api_key(&ApiKeyId::new("first"))
            .await
            .unwrap();
        store
            .set_active_api_key(&ApiKeyId::new("second"))
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

        assert!(
            store
                .delete_api_key_record(&ApiKeyId::new("second"))
                .await
                .unwrap()
        );
        assert!(store.get_active_api_key_record().await.unwrap().is_none());
        let error = store
            .set_active_api_key(&ApiKeyId::new("missing"))
            .await
            .unwrap_err();
        assert_eq!(error.kind, SecretsErrorKind::MetadataStore);
    });
}
