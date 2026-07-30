use rusqlite::Connection;

use crate::error::{DatabaseError, DatabaseResult};

mod migrations;

const DATABASE_FORMAT: &str = "atelier-workspace-database";
const DATABASE_SCHEMA_VERSION: i64 = 2;

const SCHEMA_SQL: &str = r"
CREATE TABLE atelier_schema (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    format TEXT NOT NULL,
    schema_version INTEGER NOT NULL
);

INSERT INTO atelier_schema(singleton, format, schema_version)
VALUES (1, 'atelier-workspace-database', 2);

CREATE TABLE resources (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    lifecycle TEXT NOT NULL,
    state TEXT NOT NULL,
    blob_id TEXT NOT NULL,
    mime_type TEXT,
    byte_size INTEGER,
    content_hash TEXT,
    width INTEGER,
    height INTEGER,
    created_at_ms INTEGER
);

CREATE TABLE resource_links (
    resource_id TEXT NOT NULL,
    owner_kind TEXT NOT NULL,
    owner_local_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    PRIMARY KEY (resource_id, owner_kind, owner_local_id, relation),
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE
);

CREATE INDEX idx_resource_links_owner
    ON resource_links(owner_kind, owner_local_id, resource_id);

CREATE TABLE resource_variants (
    variant_id TEXT PRIMARY KEY,
    resource_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    blob_id TEXT NOT NULL,
    mime_type TEXT,
    byte_size INTEGER,
    content_hash TEXT,
    width INTEGER,
    height INTEGER,
    created_at_ms INTEGER,
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE
);

CREATE TABLE orphan_blobs (
    blob_id TEXT PRIMARY KEY
);

CREATE TABLE generation_payloads (
    payload_ref TEXT NOT NULL,
    payload_kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (payload_kind, payload_ref)
);

CREATE INDEX idx_generation_payloads_ref
    ON generation_payloads(payload_ref);

CREATE TABLE vibe_documents (
    vibe_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    has_image INTEGER NOT NULL,
    document_json TEXT NOT NULL
);

CREATE TABLE vibe_encodings (
    cache_key TEXT PRIMARY KEY,
    vibe_id TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    information_extracted_key TEXT NOT NULL,
    record_json TEXT NOT NULL
);

CREATE INDEX idx_vibe_encodings_lookup
    ON vibe_encodings(source_hash, model, information_extracted_key);

CREATE TABLE artifacts (
    artifact_id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    primary_resource_id TEXT NOT NULL,
    primary_variant_id TEXT,
    record_json TEXT NOT NULL
);

CREATE TABLE gallery_items (
    item_id TEXT PRIMARY KEY,
    artifact_id TEXT NOT NULL,
    artifact_kind TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    safety_scan_state TEXT NOT NULL,
    manual_safety_override TEXT,
    effective_safety_label TEXT,
    indexed_at_ms INTEGER NOT NULL,
    item_json TEXT NOT NULL
);

CREATE INDEX idx_gallery_items_indexed_at
    ON gallery_items(indexed_at_ms DESC, item_id ASC);
CREATE INDEX idx_gallery_items_artifact_kind
    ON gallery_items(artifact_kind);
CREATE INDEX idx_gallery_items_source_kind
    ON gallery_items(source_kind);
CREATE INDEX idx_gallery_items_manual_safety_override
    ON gallery_items(manual_safety_override);
CREATE INDEX idx_gallery_items_effective_safety_label
    ON gallery_items(effective_safety_label);
CREATE INDEX idx_gallery_items_safety_scan_state
    ON gallery_items(safety_scan_state, indexed_at_ms, item_id);

CREATE TABLE api_key_records (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    secret_record_id TEXT NOT NULL,
    is_active INTEGER NOT NULL CHECK (is_active IN (0, 1))
);

CREATE UNIQUE INDEX idx_api_key_records_active
    ON api_key_records(is_active)
    WHERE is_active = 1;

CREATE TABLE prompt_chunks (
    chunk_id TEXT PRIMARY KEY,
    chunk_key TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    category TEXT,
    description TEXT,
    preview_resource_id TEXT,
    preview_variant_id TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX idx_prompt_chunks_key
    ON prompt_chunks(chunk_key);

CREATE TABLE prompt_presets (
    preset_id TEXT PRIMARY KEY,
    preset_kind TEXT NOT NULL,
    name TEXT NOT NULL,
    category TEXT,
    description TEXT,
    sort_order INTEGER NOT NULL,
    prompt_mode TEXT NOT NULL CHECK (prompt_mode IN ('surround', 'replace')),
    uc_mode TEXT NOT NULL CHECK (uc_mode IN ('surround', 'replace')),
    before_text TEXT NOT NULL,
    after_text TEXT NOT NULL,
    replace_text TEXT NOT NULL,
    uc_before_text TEXT NOT NULL,
    uc_after_text TEXT NOT NULL,
    uc_replace_text TEXT NOT NULL,
    quality_override TEXT,
    uc_preset_override TEXT,
    preview_resource_id TEXT,
    preview_variant_id TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE INDEX idx_prompt_presets_kind_order
    ON prompt_presets(preset_kind, sort_order, name, preset_id);

CREATE TABLE workspace_settings (
    setting_key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);

CREATE TABLE generation_queue_state (
    state_key TEXT PRIMARY KEY CHECK (state_key = 'active'),
    snapshot_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE run_history (
    run_id TEXT PRIMARY KEY,
    run_kind TEXT NOT NULL,
    run_status TEXT NOT NULL,
    batch_id TEXT,
    job_id TEXT,
    origin_run_id TEXT,
    request_index INTEGER,
    expected_samples INTEGER,
    submitted_payload_ref TEXT,
    prepared_payload_ref TEXT,
    title TEXT,
    last_error TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER,
    recoverable INTEGER NOT NULL CHECK (recoverable IN (0, 1))
);

CREATE INDEX idx_run_history_updated_at
    ON run_history(updated_at_ms DESC, run_id ASC);
CREATE INDEX idx_run_history_kind
    ON run_history(run_kind);
CREATE INDEX idx_run_history_status
    ON run_history(run_status);
CREATE INDEX idx_run_history_batch
    ON run_history(batch_id);
CREATE INDEX idx_run_history_job
    ON run_history(job_id);
CREATE INDEX idx_run_history_generation_batch_order
    ON run_history(run_kind, batch_id, request_index, run_id);

CREATE TABLE run_outputs (
    run_id TEXT NOT NULL,
    sample_index INTEGER,
    artifact_id TEXT NOT NULL,
    item_id TEXT,
    resource_id TEXT,
    variant_id TEXT,
    asset_role TEXT NOT NULL,
    variant_kind TEXT,
    output_state TEXT NOT NULL DEFAULT 'available'
        CHECK (
            (output_state = 'available' AND resource_id IS NOT NULL)
            OR output_state = 'deleted'
        ),
    FOREIGN KEY (run_id) REFERENCES run_history(run_id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_run_outputs_unique
    ON run_outputs(run_id, artifact_id, resource_id, asset_role, COALESCE(variant_id, ''))
    WHERE output_state = 'available';
CREATE INDEX idx_run_outputs_run
    ON run_outputs(run_id);
CREATE INDEX idx_run_outputs_sample
    ON run_outputs(run_id, sample_index, artifact_id);
";

pub fn initialize_or_validate_schema(connection: &mut Connection) -> DatabaseResult<()> {
    connection.execute_batch("PRAGMA foreign_keys = ON;")?;
    if database_is_empty(connection)? {
        let transaction = connection.transaction()?;
        transaction.execute_batch(SCHEMA_SQL)?;
        transaction.commit()?;
        return Ok(());
    }
    validate_or_migrate_schema(connection)
}

fn database_is_empty(connection: &Connection) -> DatabaseResult<bool> {
    connection
        .query_row(
            r"
            SELECT NOT EXISTS(
                SELECT 1
                FROM sqlite_schema
                WHERE name NOT LIKE 'sqlite_%'
            )
            ",
            [],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn validate_or_migrate_schema(connection: &mut Connection) -> DatabaseResult<()> {
    let metadata = connection.query_row(
        r"
        SELECT format, schema_version
        FROM atelier_schema
        WHERE singleton = 1
        ",
        [],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
    );
    let (format, schema_version) = metadata.map_err(|_| {
        DatabaseError::unsupported_schema("database does not contain valid Atelier schema metadata")
    })?;
    if format != DATABASE_FORMAT {
        Err(DatabaseError::unsupported_schema(format!(
            "unsupported database schema `{format}` version {schema_version}; expected \
             `{DATABASE_FORMAT}` version {DATABASE_SCHEMA_VERSION}"
        )))
    } else if schema_version > DATABASE_SCHEMA_VERSION {
        Err(DatabaseError::unsupported_schema(format!(
            "database schema version {schema_version} is newer than supported version \
             {DATABASE_SCHEMA_VERSION}"
        )))
    } else {
        migrations::migrate(connection, schema_version, DATABASE_SCHEMA_VERSION)
    }
}
