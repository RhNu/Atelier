use rusqlite::{Connection, params};

use crate::error::DatabaseResult;

const CURRENT_SCHEMA_VERSION: i64 = 2;
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

    let current_applied = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
        params![CURRENT_SCHEMA_VERSION],
        |row| row.get::<_, bool>(0),
    )?;
    if current_applied {
        connection.execute_batch(API_KEY_REGISTRY_SQL)?;
        connection.execute_batch(PROMPT_RESOURCES_SQL)?;
        return Ok(());
    }

    let v1_applied = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 1)",
        [],
        |row| row.get::<_, bool>(0),
    )?;

    let tx = connection.transaction()?;
    if !v1_applied {
        tx.execute_batch(SCHEMA_SQL)?;
        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (1)",
            [],
        )?;
    }
    tx.execute_batch(API_KEY_REGISTRY_SQL)?;
    tx.execute_batch(PROMPT_RESOURCES_SQL)?;
    tx.execute(
        "INSERT INTO schema_migrations(version) VALUES (?1)",
        params![CURRENT_SCHEMA_VERSION],
    )?;
    tx.commit()?;
    Ok(())
}
