use atelier_adapter_database::{DatabaseConnection, DatabaseRunHistoryRepository};
use atelier_jobs::{RunHistoryRepository, RunOutputState};
use futures_executor::block_on;
use rusqlite::Connection;

#[test]
fn v10_normalizes_tombstones_and_gallery_metadata_projection() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_ms INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO schema_migrations(version)
                VALUES (1), (2), (3), (4), (5), (6), (7), (8), (9);
                CREATE TABLE run_history (
                    run_id TEXT PRIMARY KEY,
                    run_kind TEXT NOT NULL, run_status TEXT NOT NULL,
                    batch_id TEXT, job_id TEXT, origin_run_id TEXT,
                    request_index INTEGER, expected_samples INTEGER,
                    submitted_payload_ref TEXT, prepared_payload_ref TEXT,
                    title TEXT, last_error TEXT,
                    created_at_ms INTEGER NOT NULL, updated_at_ms INTEGER NOT NULL,
                    completed_at_ms INTEGER,
                    recoverable INTEGER NOT NULL
                );
                CREATE TABLE run_outputs (
                    run_id TEXT NOT NULL,
                    sample_index INTEGER, artifact_id TEXT NOT NULL, item_id TEXT,
                    resource_id TEXT NOT NULL,
                    variant_id TEXT, asset_role TEXT NOT NULL, variant_kind TEXT,
                    output_state TEXT NOT NULL
                );
                INSERT INTO run_history VALUES (
                    'job-old', 'generation', 'succeeded', 'batch-old', 'job-old', NULL,
                    0, 1, NULL, NULL, NULL, NULL, 1, 1, 1, 0
                );
                INSERT INTO run_outputs VALUES (
                    'job-old', 0, 'artifact-old', 'gallery-old', 'resource-old',
                    'preview', 'original', 'original', 'deleted'
                );
                CREATE TABLE artifacts (
                    artifact_id TEXT PRIMARY KEY,
                    artifact_kind TEXT NOT NULL,
                    source_kind TEXT NOT NULL,
                    primary_resource_id TEXT NOT NULL,
                    primary_variant_id TEXT,
                    record_json TEXT NOT NULL
                );
                INSERT INTO artifacts VALUES (
                    'artifact-old', 'generated_image', 'generation', 'resource-old', NULL,
                    '{"replay":{"prompt_snapshot":"legacy prompt"}}'
                );
                CREATE TABLE gallery_items (
                    item_id TEXT PRIMARY KEY,
                    artifact_id TEXT NOT NULL,
                    artifact_kind TEXT NOT NULL,
                    source_kind TEXT NOT NULL,
                    manual_safety_override TEXT,
                    indexed_at_ms INTEGER NOT NULL,
                    item_json TEXT NOT NULL,
                    effective_safety_label TEXT
                );
                INSERT INTO gallery_items VALUES (
                    'gallery-old', 'artifact-old', 'generated_image', 'generation', NULL, 1,
                    '{"metadata":{"embedded_metadata_json":"heavy"},"replay":null}', NULL
                );
                "#,
            )
            .unwrap();
    }

    let repository = DatabaseRunHistoryRepository::new(DatabaseConnection::open(&path).unwrap());
    block_on(async {
        let tombstone = repository
            .list_run_outputs("job-old")
            .await
            .unwrap()
            .remove(0);
        assert_eq!(tombstone.state, RunOutputState::Deleted);
        assert_eq!(tombstone.item_id, None);
        assert_eq!(tombstone.resource_id, None);
        assert_eq!(tombstone.variant_id, None);
    });
    let connection = Connection::open(&path).unwrap();
    let (raw_metadata, prompt): (Option<String>, Option<String>) = connection
        .query_row(
            r"
            SELECT
                json_extract(item_json, '$.metadata.embedded_metadata_json'),
                json_extract(item_json, '$.replay.prompt_snapshot')
            FROM gallery_items
            WHERE item_id = 'gallery-old'
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(raw_metadata, None);
    assert_eq!(prompt.as_deref(), Some("legacy prompt"));
}
