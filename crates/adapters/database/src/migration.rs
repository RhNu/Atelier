use rusqlite::{Connection, params};

use crate::error::DatabaseResult;

const CURRENT_SCHEMA_VERSION: i64 = 10;
const API_KEY_REGISTRY_SQL: &str = r"
CREATE TABLE IF NOT EXISTS api_key_records (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    secret_record_id TEXT NOT NULL,
    is_active INTEGER NOT NULL CHECK (is_active IN (0, 1))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_api_key_records_active
    ON api_key_records(is_active)
    WHERE is_active = 1;
";

const SCHEMA_SQL: &str = r"
CREATE TABLE IF NOT EXISTS resources (
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

CREATE TABLE IF NOT EXISTS resource_links (
    resource_id TEXT NOT NULL,
    owner_kind TEXT NOT NULL,
    owner_local_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    PRIMARY KEY (resource_id, owner_kind, owner_local_id, relation),
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_resource_links_owner
    ON resource_links(owner_kind, owner_local_id, resource_id);

CREATE TABLE IF NOT EXISTS resource_variants (
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

CREATE TABLE IF NOT EXISTS orphan_blobs (
    blob_id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS generation_payloads (
    payload_ref TEXT NOT NULL,
    payload_kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (payload_kind, payload_ref)
);

CREATE INDEX IF NOT EXISTS idx_generation_payloads_ref
    ON generation_payloads(payload_ref);

CREATE TABLE IF NOT EXISTS vibe_documents (
    vibe_id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    has_image INTEGER NOT NULL,
    document_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS vibe_encodings (
    cache_key TEXT PRIMARY KEY,
    vibe_id TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    model TEXT NOT NULL,
    information_extracted_key TEXT NOT NULL,
    record_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_vibe_encodings_lookup
    ON vibe_encodings(source_hash, model, information_extracted_key);

CREATE TABLE IF NOT EXISTS artifacts (
    artifact_id TEXT PRIMARY KEY,
    artifact_kind TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    primary_resource_id TEXT NOT NULL,
    primary_variant_id TEXT,
    record_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gallery_items (
    item_id TEXT PRIMARY KEY,
    artifact_id TEXT NOT NULL,
    artifact_kind TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    manual_safety_override TEXT,
    indexed_at_ms INTEGER NOT NULL,
    item_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_gallery_items_indexed_at
    ON gallery_items(indexed_at_ms DESC, item_id ASC);
CREATE INDEX IF NOT EXISTS idx_gallery_items_artifact_kind
    ON gallery_items(artifact_kind);
CREATE INDEX IF NOT EXISTS idx_gallery_items_source_kind
    ON gallery_items(source_kind);
CREATE INDEX IF NOT EXISTS idx_gallery_items_manual_safety_override
    ON gallery_items(manual_safety_override);
";

const PROMPT_RESOURCES_SQL: &str = r"
CREATE TABLE IF NOT EXISTS prompt_chunks (
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

CREATE INDEX IF NOT EXISTS idx_prompt_chunks_key
    ON prompt_chunks(chunk_key);

CREATE TABLE IF NOT EXISTS prompt_presets (
    preset_id TEXT PRIMARY KEY,
    preset_kind TEXT NOT NULL,
    name TEXT NOT NULL,
    category TEXT,
    description TEXT,
    sort_order INTEGER NOT NULL,
    enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
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

CREATE INDEX IF NOT EXISTS idx_prompt_presets_kind_order
    ON prompt_presets(preset_kind, sort_order, name, preset_id);
";

const PROMPT_PRESET_BEHAVIOR_SQL: &str = r"
ALTER TABLE prompt_presets ADD COLUMN prompt_mode TEXT NOT NULL DEFAULT 'surround'
    CHECK (prompt_mode IN ('surround', 'replace'));
ALTER TABLE prompt_presets ADD COLUMN uc_mode TEXT NOT NULL DEFAULT 'surround'
    CHECK (uc_mode IN ('surround', 'replace'));

UPDATE prompt_presets
SET prompt_mode = CASE
        WHEN trim(replace_text) = '' THEN 'surround'
        ELSE 'replace'
    END,
    uc_mode = CASE
        WHEN trim(uc_replace_text) = '' THEN 'surround'
        ELSE 'replace'
    END;
";

const SETTINGS_SQL: &str = r"
CREATE TABLE IF NOT EXISTS workspace_settings (
    setting_key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL
);
";

const JOB_HISTORY_SQL: &str = r"
CREATE TABLE IF NOT EXISTS generation_queue_state (
    state_key TEXT PRIMARY KEY CHECK (state_key = 'active'),
    snapshot_json TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS run_history (
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

CREATE INDEX IF NOT EXISTS idx_run_history_updated_at
    ON run_history(updated_at_ms DESC, run_id ASC);
CREATE INDEX IF NOT EXISTS idx_run_history_kind
    ON run_history(run_kind);
CREATE INDEX IF NOT EXISTS idx_run_history_status
    ON run_history(run_status);
CREATE INDEX IF NOT EXISTS idx_run_history_batch
    ON run_history(batch_id);
CREATE INDEX IF NOT EXISTS idx_run_history_job
    ON run_history(job_id);
CREATE TABLE IF NOT EXISTS run_outputs (
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

CREATE UNIQUE INDEX IF NOT EXISTS idx_run_outputs_unique
    ON run_outputs(run_id, artifact_id, resource_id, asset_role, COALESCE(variant_id, ''));
CREATE INDEX IF NOT EXISTS idx_run_outputs_run
    ON run_outputs(run_id);
";

const GENERATION_BATCH_HISTORY_INDEX_SQL: &str = r"
CREATE INDEX IF NOT EXISTS idx_run_history_generation_batch_order
    ON run_history(run_kind, batch_id, request_index, run_id);
CREATE INDEX IF NOT EXISTS idx_run_outputs_sample
    ON run_outputs(run_id, sample_index, artifact_id);
";

const RUN_OUTPUT_LIFECYCLE_SQL: &str = r"
ALTER TABLE run_outputs ADD COLUMN output_state TEXT NOT NULL DEFAULT 'available'
    CHECK (output_state IN ('available', 'deleted'));
";

const GENERATION_BATCH_HISTORY_SQL: &str = r"
ALTER TABLE run_history ADD COLUMN request_index INTEGER;
ALTER TABLE run_history ADD COLUMN expected_samples INTEGER;
ALTER TABLE run_outputs ADD COLUMN sample_index INTEGER;

UPDATE run_history
SET request_index = (
    SELECT COUNT(*)
    FROM run_history AS earlier
    WHERE earlier.run_kind = 'generation'
        AND earlier.batch_id = run_history.batch_id
        AND (
            earlier.created_at_ms < run_history.created_at_ms
            OR (
                earlier.created_at_ms = run_history.created_at_ms
                AND earlier.run_id < run_history.run_id
            )
        )
)
WHERE run_kind = 'generation' AND batch_id IS NOT NULL AND request_index IS NULL;

UPDATE run_outputs
SET sample_index = COALESCE(
    (
        SELECT CAST(json_extract(gallery_items.item_json, '$.metadata.sample_index') AS INTEGER)
        FROM gallery_items
        WHERE gallery_items.item_id = run_outputs.item_id
    ),
    0
)
WHERE sample_index IS NULL;

UPDATE run_history
SET expected_samples = MAX(
    1,
    COALESCE(
        (
            SELECT COALESCE(
                CAST(json_extract(payload.payload_json, '$.request.request.base.n_samples') AS INTEGER),
                CAST(json_extract(payload.payload_json, '$.request.request.n_samples') AS INTEGER)
            )
            FROM generation_payloads AS payload
            WHERE payload.payload_kind = 'submitted'
                AND payload.payload_ref = run_history.submitted_payload_ref
        ),
        (
            SELECT MAX(run_outputs.sample_index) + 1
            FROM run_outputs
            WHERE run_outputs.run_id = run_history.run_id
        ),
        1
    )
)
WHERE run_kind = 'generation' AND expected_samples IS NULL;

";

const GALLERY_EFFECTIVE_SAFETY_SQL: &str = r"
ALTER TABLE gallery_items ADD COLUMN effective_safety_label TEXT;

UPDATE gallery_items
SET effective_safety_label = CASE
    WHEN manual_safety_override IS NOT NULL THEN manual_safety_override
    WHEN json_extract(item_json, '$.safety_assessment.score') >= 0.8 THEN 'sensitive'
    WHEN json_extract(item_json, '$.safety_assessment.score') IS NOT NULL THEN 'safe'
    ELSE NULL
END;

CREATE INDEX idx_gallery_items_effective_safety_label
    ON gallery_items(effective_safety_label);
";

const RUN_OUTPUT_TOMBSTONES_SQL: &str = r"
CREATE TABLE run_outputs_v10 (
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

INSERT INTO run_outputs_v10(
    run_id,
    sample_index,
    artifact_id,
    item_id,
    resource_id,
    variant_id,
    asset_role,
    variant_kind,
    output_state
)
SELECT
    run_id,
    sample_index,
    artifact_id,
    CASE WHEN output_state = 'deleted' THEN NULL ELSE item_id END,
    CASE WHEN output_state = 'deleted' THEN NULL ELSE resource_id END,
    CASE WHEN output_state = 'deleted' THEN NULL ELSE variant_id END,
    asset_role,
    variant_kind,
    output_state
FROM run_outputs;

DROP TABLE run_outputs;
ALTER TABLE run_outputs_v10 RENAME TO run_outputs;

CREATE UNIQUE INDEX idx_run_outputs_unique
    ON run_outputs(run_id, artifact_id, resource_id, asset_role, COALESCE(variant_id, ''))
    WHERE output_state = 'available';
CREATE INDEX idx_run_outputs_run
    ON run_outputs(run_id);
CREATE INDEX idx_run_outputs_sample
    ON run_outputs(run_id, sample_index, artifact_id);

UPDATE gallery_items
SET item_json = json_remove(item_json, '$.metadata.embedded_metadata_json')
WHERE json_extract(item_json, '$.metadata.embedded_metadata_json') IS NOT NULL;

UPDATE gallery_items
SET item_json = json_set(
    item_json,
    '$.replay',
    (
        SELECT json(json_extract(artifacts.record_json, '$.replay'))
        FROM artifacts
        WHERE artifacts.artifact_id = gallery_items.artifact_id
    )
)
WHERE json_extract(item_json, '$.replay') IS NULL
  AND EXISTS (
      SELECT 1
      FROM artifacts
      WHERE artifacts.artifact_id = gallery_items.artifact_id
        AND json_extract(artifacts.record_json, '$.replay') IS NOT NULL
  );
";

pub fn run_migrations(connection: &mut Connection) -> DatabaseResult<()> {
    connection.execute_batch(
        r"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at_ms INTEGER NOT NULL DEFAULT (
                CAST(strftime('%s', 'now') AS INTEGER) * 1000
            )
        );
        ",
    )?;

    let current_applied = migration_applied(connection, CURRENT_SCHEMA_VERSION)?;
    if current_applied {
        connection.execute_batch(API_KEY_REGISTRY_SQL)?;
        connection.execute_batch(PROMPT_RESOURCES_SQL)?;
        connection.execute_batch(SETTINGS_SQL)?;
        connection.execute_batch(JOB_HISTORY_SQL)?;
        if !column_exists(connection, "run_outputs", "output_state")? {
            connection.execute_batch(RUN_OUTPUT_LIFECYCLE_SQL)?;
        }
        connection.execute_batch(GENERATION_BATCH_HISTORY_INDEX_SQL)?;
        return Ok(());
    }

    let v1_applied = migration_applied(connection, 1)?;
    let v2_applied = migration_applied(connection, 2)?;
    let v3_applied = migration_applied(connection, 3)?;
    let v4_applied = migration_applied(connection, 4)?;
    let v5_applied = migration_applied(connection, 5)?;
    let v6_applied = migration_applied(connection, 6)?;
    let v7_applied = migration_applied(connection, 7)?;

    let tx = connection.transaction()?;
    // Recreate any missing idempotent v1 objects as well as trusting the marker. This also
    // repairs partially initialized early-development databases.
    tx.execute_batch(SCHEMA_SQL)?;
    if !v1_applied {
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (1)",
            [],
        )?;
    }
    tx.execute_batch(API_KEY_REGISTRY_SQL)?;
    tx.execute_batch(PROMPT_RESOURCES_SQL)?;
    if !v2_applied {
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (2)",
            [],
        )?;
    }
    tx.execute_batch(SETTINGS_SQL)?;
    if !v3_applied {
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (3)",
            [],
        )?;
    }
    tx.execute_batch(JOB_HISTORY_SQL)?;
    if !v4_applied {
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (4)",
            [],
        )?;
    }
    if !v5_applied {
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (5)",
            [],
        )?;
    }
    if !v6_applied {
        tx.execute_batch(GALLERY_EFFECTIVE_SAFETY_SQL)?;
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (6)",
            [],
        )?;
    }
    if v4_applied && !v7_applied {
        tx.execute_batch(GENERATION_BATCH_HISTORY_SQL)?;
    }
    if !column_exists(&tx, "prompt_presets", "prompt_mode")? {
        tx.execute_batch(PROMPT_PRESET_BEHAVIOR_SQL)?;
    }
    if !column_exists(&tx, "run_outputs", "output_state")? {
        tx.execute_batch(RUN_OUTPUT_LIFECYCLE_SQL)?;
    }
    tx.execute_batch(RUN_OUTPUT_TOMBSTONES_SQL)?;
    tx.execute(
        "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
        params![CURRENT_SCHEMA_VERSION],
    )?;
    tx.commit()?;
    Ok(())
}

fn migration_applied(connection: &Connection, version: i64) -> DatabaseResult<bool> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            params![version],
            |row| row.get(0),
        )
        .map_err(Into::into)
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> DatabaseResult<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}
