use std::collections::{BTreeMap, BTreeSet};

use atelier_resource_library::{LibraryFolderId, LibraryResourceId, ResourceIdentifier};
use rusqlite::{Transaction, params};

use crate::error::DatabaseResult;

const TO_VERSION: i64 = 5;

const CREATE_LIBRARY_SQL: &str = r"
CREATE TABLE resource_library_nodes (
    node_id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL CHECK (
        namespace IN ('prompt_chunk', 'main_preset', 'character_preset', 'vibe')
    ),
    parent_folder_id TEXT,
    node_kind TEXT NOT NULL CHECK (node_kind IN ('folder', 'resource')),
    owner_local_id TEXT,
    identifier TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    CHECK (
        (node_kind = 'folder' AND owner_local_id IS NULL) OR
        (node_kind = 'resource' AND owner_local_id IS NOT NULL)
    ),
    FOREIGN KEY (parent_folder_id) REFERENCES resource_library_nodes(node_id) ON DELETE RESTRICT,
    UNIQUE (namespace, owner_local_id)
);

CREATE UNIQUE INDEX idx_resource_library_sibling_identifier
    ON resource_library_nodes(namespace, COALESCE(parent_folder_id, ''), identifier);
CREATE INDEX idx_resource_library_parent
    ON resource_library_nodes(namespace, parent_folder_id, node_kind, identifier);

CREATE TABLE resource_library_aliases (
    node_id TEXT NOT NULL,
    alias_order INTEGER NOT NULL,
    alias TEXT NOT NULL,
    PRIMARY KEY (node_id, alias_order),
    UNIQUE (node_id, alias),
    FOREIGN KEY (node_id) REFERENCES resource_library_nodes(node_id) ON DELETE CASCADE
);
";

pub(super) fn migrate(connection: &mut rusqlite::Connection) -> DatabaseResult<i64> {
    let transaction = connection.transaction()?;
    transaction.execute_batch(CREATE_LIBRARY_SQL)?;
    backfill_library(&transaction)?;
    transaction.execute(
        "UPDATE atelier_schema SET schema_version = ?1 WHERE singleton = 1",
        [TO_VERSION],
    )?;
    transaction.commit()?;
    Ok(TO_VERSION)
}

#[derive(Clone, Debug)]
struct LegacyResource {
    namespace: &'static str,
    owner_id: String,
    display_name: String,
    category: Option<String>,
    created_at_ms: i64,
    updated_at_ms: i64,
}

fn backfill_library(transaction: &Transaction<'_>) -> DatabaseResult<()> {
    let resources = legacy_resources(transaction)?;
    let mut used = BTreeSet::new();
    let mut folders = BTreeMap::new();

    for resource in &resources {
        let Some(category) = normalized_category(resource.category.as_deref()) else {
            continue;
        };
        let key = (resource.namespace, category.clone());
        if folders.contains_key(&key) {
            continue;
        }
        let identifier = unique_identifier(resource.namespace, None, &category, &mut used);
        let folder_id = LibraryFolderId::allocate();
        transaction.execute(
            r"
            INSERT INTO resource_library_nodes(
                node_id, namespace, parent_folder_id, node_kind, owner_local_id,
                identifier, display_name, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, NULL, 'folder', NULL, ?3, ?4, ?5, ?6)
            ",
            params![
                folder_id.as_str(),
                resource.namespace,
                identifier,
                category,
                resource.created_at_ms,
                resource.updated_at_ms,
            ],
        )?;
        folders.insert(key, folder_id.as_str().to_owned());
    }

    for resource in resources {
        let category = normalized_category(resource.category.as_deref());
        let parent_id = category
            .as_ref()
            .and_then(|category| folders.get(&(resource.namespace, category.clone())))
            .cloned();
        let identifier = unique_identifier(
            resource.namespace,
            parent_id.as_deref(),
            &resource.display_name,
            &mut used,
        );
        let resource_id = LibraryResourceId::allocate();
        transaction.execute(
            r"
            INSERT INTO resource_library_nodes(
                node_id, namespace, parent_folder_id, node_kind, owner_local_id,
                identifier, display_name, created_at_ms, updated_at_ms
            ) VALUES (?1, ?2, ?3, 'resource', ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                resource_id.as_str(),
                resource.namespace,
                parent_id,
                resource.owner_id,
                identifier,
                resource.display_name,
                resource.created_at_ms,
                resource.updated_at_ms,
            ],
        )?;
    }
    Ok(())
}

fn legacy_resources(transaction: &Transaction<'_>) -> DatabaseResult<Vec<LegacyResource>> {
    let mut resources = Vec::new();
    collect_rows(
        transaction,
        "SELECT chunk_id, chunk_key, category, created_at_ms, updated_at_ms FROM prompt_chunks ORDER BY chunk_id",
        "prompt_chunk",
        &mut resources,
    )?;
    for (kind, namespace) in [("main", "main_preset"), ("character", "character_preset")] {
        let mut statement = transaction.prepare(
            "SELECT preset_id, name, category, created_at_ms, updated_at_ms FROM prompt_presets WHERE preset_kind = ?1 ORDER BY preset_id",
        )?;
        let rows = statement.query_map([kind], |row| {
            Ok(LegacyResource {
                namespace,
                owner_id: row.get(0)?,
                display_name: row.get(1)?,
                category: row.get(2)?,
                created_at_ms: row.get(3)?,
                updated_at_ms: row.get(4)?,
            })
        })?;
        resources.extend(rows.collect::<Result<Vec<_>, _>>()?);
    }
    let mut statement =
        transaction.prepare("SELECT vibe_id, display_name FROM vibe_documents ORDER BY vibe_id")?;
    let rows = statement.query_map([], |row| {
        Ok(LegacyResource {
            namespace: "vibe",
            owner_id: row.get(0)?,
            display_name: row.get(1)?,
            category: None,
            created_at_ms: 0,
            updated_at_ms: 0,
        })
    })?;
    resources.extend(rows.collect::<Result<Vec<_>, _>>()?);
    Ok(resources)
}

fn collect_rows(
    transaction: &Transaction<'_>,
    sql: &str,
    namespace: &'static str,
    resources: &mut Vec<LegacyResource>,
) -> DatabaseResult<()> {
    let mut statement = transaction.prepare(sql)?;
    let rows = statement.query_map([], |row| {
        Ok(LegacyResource {
            namespace,
            owner_id: row.get(0)?,
            display_name: row.get(1)?,
            category: row.get(2)?,
            created_at_ms: row.get(3)?,
            updated_at_ms: row.get(4)?,
        })
    })?;
    resources.extend(rows.collect::<Result<Vec<_>, _>>()?);
    Ok(())
}

fn normalized_category(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn unique_identifier(
    namespace: &str,
    parent_id: Option<&str>,
    source: &str,
    used: &mut BTreeSet<(String, Option<String>, String)>,
) -> String {
    let base = ResourceIdentifier::from_legacy(source, "item")
        .as_str()
        .to_owned();
    let mut candidate = base.clone();
    let mut suffix = 2;
    while !used.insert((
        namespace.to_owned(),
        parent_id.map(str::to_owned),
        candidate.clone(),
    )) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    candidate
}
