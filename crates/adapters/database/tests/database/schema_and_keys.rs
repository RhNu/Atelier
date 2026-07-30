use super::*;
use atelier_adapter_database::DatabaseErrorKind;

#[test]
fn schema_initializes_once_and_file_backed_database_reopens() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let first = DatabaseConnection::open(&path).unwrap();
        let repository = DatabaseResourceCatalogRepository::new(first.clone());
        let catalog =
            ResourceCatalog::new(repository, MemoryBlobStore::default(), NullVariantBuilder);

        let reference = catalog
            .register_resource(generated_resource("persisted-res", vec![1, 2, 3]))
            .await
            .unwrap();
        drop(catalog);
        drop(first);

        let raw = Connection::open(&path).unwrap();
        let metadata: (String, i64) = raw
            .query_row(
                "SELECT format, schema_version FROM atelier_schema WHERE singleton = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(metadata, ("atelier-workspace-database".to_owned(), 2));
        drop(raw);

        let reopened = DatabaseConnection::open(&path).unwrap();
        let repository = DatabaseResourceCatalogRepository::new(reopened);
        let record = repository.get_ready_record(&reference.id).await.unwrap();

        assert_eq!(record.unwrap().metadata.byte_size, Some(3));
    });
}

#[test]
fn version_one_database_migrates_at_a_single_testable_boundary() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let connection = DatabaseConnection::open(&path).unwrap();
        let gallery = DatabaseGalleryIndex::new(connection.clone());
        let artifact = artifact_record(
            "legacy-safety",
            11,
            ArtifactSource::GenerationJob {
                job_id: "job-1".to_owned(),
                batch_id: None,
            },
        );
        let item = GalleryItem {
            id: GalleryItemId::from_artifact_id(&artifact.id),
            artifact_id: artifact.id.clone(),
            artifact_kind: artifact.kind,
            source: artifact.source,
            primary_resource: artifact.primary_resource.clone(),
            assets: artifact.assets,
            metadata: artifact.metadata,
            replay: artifact.replay,
            safety: GallerySafetyState::Scanned(Box::new(test_safety_assessment(
                artifact.primary_resource,
                ImageSafetyScore::new(0.9).unwrap(),
            ))),
            manual_safety_override: Some(GallerySafetyOverride::Safe),
            indexed_at_ms: 100,
        };
        gallery.upsert_item(item.clone()).await.unwrap();
        drop(gallery);
        drop(connection);

        let raw = Connection::open(&path).unwrap();
        let text: String = raw
            .query_row(
                "SELECT item_json FROM gallery_items WHERE item_id = ?1",
                [item.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let mut legacy: serde_json::Value = serde_json::from_str(&text).unwrap();
        legacy["schema_version"] = serde_json::json!(1);
        let safety = legacy.as_object_mut().unwrap().remove("safety").unwrap();
        legacy["safety_assessment"] = safety["assessment"].clone();
        raw.execute(
            "UPDATE gallery_items SET item_json = ?2 WHERE item_id = ?1",
            rusqlite::params![item.id.as_str(), serde_json::to_string(&legacy).unwrap()],
        )
        .unwrap();
        raw.execute_batch(
            r"
            DROP INDEX idx_gallery_items_safety_scan_state;
            ALTER TABLE gallery_items DROP COLUMN safety_scan_state;
            UPDATE atelier_schema SET schema_version = 1 WHERE singleton = 1;
            ",
        )
        .unwrap();
        drop(raw);

        let migrated = DatabaseConnection::open(&path).unwrap();
        let gallery = DatabaseGalleryIndex::new(migrated.clone());
        let migrated_item = gallery.get_item(&item.id).await.unwrap().unwrap();
        assert_eq!(
            migrated_item.safety,
            GallerySafetyState::Unavailable {
                message: "legacy safety assessment requires rescan".to_owned()
            }
        );
        assert_eq!(
            migrated_item.manual_safety_override,
            Some(GallerySafetyOverride::Safe)
        );
        drop(gallery);
        drop(migrated);

        let raw = Connection::open(&path).unwrap();
        assert_eq!(
            raw.query_row(
                "SELECT schema_version FROM atelier_schema WHERE singleton = 1",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
        let row: (String, Option<String>) = raw
            .query_row(
                "SELECT safety_scan_state, effective_safety_label FROM gallery_items \
                 WHERE item_id = ?1",
                [item.id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(row, ("unavailable".to_owned(), Some("safe".to_owned())));
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
fn old_migration_database_is_rejected_without_changes() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch(
        r"
        CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL DEFAULT 0
        );
        INSERT INTO schema_migrations(version) VALUES (1), (10);
        CREATE TABLE sentinel(value TEXT NOT NULL);
        INSERT INTO sentinel(value) VALUES ('unchanged');
        ",
    )
    .unwrap();
    drop(raw);

    let error = DatabaseConnection::open(&path).unwrap_err();
    assert_eq!(error.kind(), DatabaseErrorKind::UnsupportedSchema);

    let raw = Connection::open(&path).unwrap();
    assert_eq!(
        raw.query_row("SELECT value FROM sentinel", [], |row| row
            .get::<_, String>(0))
            .unwrap(),
        "unchanged"
    );
    let has_new_metadata: bool = raw
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name = 'atelier_schema')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!has_new_metadata);
}

#[test]
fn database_rejects_unknown_format_and_non_current_versions() {
    for (format, version) in [
        ("atelier-workspace-database", 0),
        ("atelier-workspace-database", 3),
        ("another-database", 2),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        drop(DatabaseConnection::open(&path).unwrap());
        let raw = Connection::open(&path).unwrap();
        raw.execute(
            "UPDATE atelier_schema SET format = ?1, schema_version = ?2 WHERE singleton = 1",
            rusqlite::params![format, version],
        )
        .unwrap();
        drop(raw);

        let error = DatabaseConnection::open(&path).unwrap_err();
        assert_eq!(error.kind(), DatabaseErrorKind::UnsupportedSchema);
    }
}

#[test]
fn empty_database_file_initializes_but_unmarked_nonempty_database_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let empty_path = temp.path().join("empty.sqlite3");
    std::fs::File::create(&empty_path).unwrap();
    drop(DatabaseConnection::open(&empty_path).unwrap());

    let unmarked_path = temp.path().join("unmarked.sqlite3");
    let raw = Connection::open(&unmarked_path).unwrap();
    raw.execute("CREATE TABLE data(value TEXT NOT NULL)", [])
        .unwrap();
    drop(raw);
    let error = DatabaseConnection::open(&unmarked_path).unwrap_err();
    assert_eq!(error.kind(), DatabaseErrorKind::UnsupportedSchema);
}

#[test]
fn current_prompt_preset_schema_has_no_legacy_enabled_column() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    drop(DatabaseConnection::open(&path).unwrap());
    let raw = Connection::open(path).unwrap();
    let mut statement = raw.prepare("PRAGMA table_info(prompt_presets)").unwrap();
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert!(columns.contains(&"prompt_mode".to_owned()));
    assert!(columns.contains(&"uc_mode".to_owned()));
    assert!(!columns.contains(&"enabled".to_owned()));
}

#[test]
fn artifact_warnings_use_only_the_current_structured_shape() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let connection = DatabaseConnection::open(&path).unwrap();
        let repository = DatabaseArtifactRepository::new(connection.clone());
        let mut record = artifact_record(
            "warning",
            1,
            ArtifactSource::DirectorRun {
                run_id: "run-1".to_owned(),
            },
        );
        record.metadata.embedded_metadata_warnings = vec![EmbeddedMetadataWarning::Unknown(
            "future-warning".to_owned(),
        )];
        repository.insert_artifact(record.clone()).await.unwrap();
        drop(repository);
        drop(connection);

        let raw = Connection::open(&path).unwrap();
        let json: String = raw
            .query_row(
                "SELECT record_json FROM artifacts WHERE artifact_id = 'warning'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            value["metadata"]["embedded_metadata_warnings"][0],
            serde_json::json!({
                "code": "unknown",
                "message": "future-warning"
            })
        );

        let mut legacy = value;
        legacy["metadata"]["embedded_metadata_warnings"] = serde_json::json!(["future-warning"]);
        raw.execute(
            "UPDATE artifacts SET record_json = ?1 WHERE artifact_id = 'warning'",
            [serde_json::to_string(&legacy).unwrap()],
        )
        .unwrap();
        drop(raw);

        let repository = DatabaseArtifactRepository::new(DatabaseConnection::open(&path).unwrap());
        assert!(repository.get_artifact(&record.id).await.is_err());
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
