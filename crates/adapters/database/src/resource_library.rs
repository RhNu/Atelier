#![allow(clippy::needless_pass_by_value, clippy::significant_drop_tightening)]

use std::collections::BTreeMap;

use async_trait::async_trait;
use atelier_prompt_resources::{
    PromptChunkKey, chunk_references_in_text, rewrite_chunk_references,
};
use atelier_resource_library::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    ResourceIdentifier, ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryRepository,
    ResourceLibraryResult, ResourceName,
};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use crate::{DatabaseConnection, DatabaseError};

#[derive(Clone, Debug)]
pub struct DatabaseResourceLibraryRepository {
    connection: DatabaseConnection,
}

impl DatabaseResourceLibraryRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    fn load_folders(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<Vec<LibraryFolder>> {
        let connection = self.connection.lock().map_err(repository_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT node_id, parent_folder_id, identifier, display_name,
                       created_at_ms, updated_at_ms
                FROM resource_library_nodes
                WHERE namespace = ?1 AND node_kind = 'folder'
                ORDER BY identifier, node_id
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map([namespace.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(sql_error)?;
        rows.map(|row| {
            let (id, parent_id, identifier, display_name, created_at_ms, updated_at_ms) =
                row.map_err(sql_error)?;
            Ok(LibraryFolder {
                id: LibraryFolderId::parse(&id)?,
                namespace,
                parent_id: parent_id
                    .as_deref()
                    .map(LibraryFolderId::parse)
                    .transpose()?,
                identifier: ResourceIdentifier::parse(&identifier)?,
                display_name,
                created_at_ms: to_u64(created_at_ms)?,
                updated_at_ms: to_u64(updated_at_ms)?,
            })
        })
        .collect()
    }

    fn load_resources(
        &self,
        namespace: Option<LibraryNamespace>,
    ) -> ResourceLibraryResult<Vec<LibraryResource>> {
        let connection = self.connection.lock().map_err(repository_error)?;
        let aliases = load_aliases(&connection)?;
        let (sql, namespace_text) = namespace.map_or_else(
            || (
                "SELECT node_id, namespace, parent_folder_id, identifier, display_name, created_at_ms, updated_at_ms FROM resource_library_nodes WHERE node_kind = 'resource' ORDER BY identifier, node_id".to_owned(),
                None,
            ),
            |namespace| (
                "SELECT node_id, namespace, parent_folder_id, identifier, display_name, created_at_ms, updated_at_ms FROM resource_library_nodes WHERE node_kind = 'resource' AND namespace = ?1 ORDER BY identifier, node_id".to_owned(),
                Some(namespace.as_str()),
            ),
        );
        let mut statement = connection.prepare(&sql).map_err(sql_error)?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        };
        let values = if let Some(namespace_text) = namespace_text {
            statement
                .query_map([namespace_text], map_row)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?
        } else {
            statement
                .query_map([], map_row)
                .map_err(sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error)?
        };
        values
            .into_iter()
            .map(
                |(id, namespace, folder_id, identifier, display_name, created, updated)| {
                    Ok(LibraryResource {
                        id: LibraryResourceId::parse(&id)?,
                        namespace: namespace_from_str(&namespace)?,
                        folder_id: folder_id
                            .as_deref()
                            .map(LibraryFolderId::parse)
                            .transpose()?,
                        name: ResourceName::new(
                            ResourceIdentifier::parse(&identifier)?,
                            &display_name,
                            aliases.get(&id).cloned().unwrap_or_default(),
                        )?,
                        created_at_ms: to_u64(created)?,
                        updated_at_ms: to_u64(updated)?,
                    })
                },
            )
            .collect()
    }
}

#[async_trait]
impl ResourceLibraryRepository for DatabaseResourceLibraryRepository {
    async fn list_folders(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<Vec<LibraryFolder>> {
        self.load_folders(namespace)
    }

    async fn get_resource(
        &self,
        id: &LibraryResourceId,
    ) -> ResourceLibraryResult<Option<LibraryResource>> {
        Ok(self
            .load_resources(None)?
            .into_iter()
            .find(|resource| &resource.id == id))
    }

    async fn list_resources(
        &self,
        namespace: LibraryNamespace,
    ) -> ResourceLibraryResult<Vec<LibraryResource>> {
        self.load_resources(Some(namespace))
    }

    async fn save_folder(&self, folder: LibraryFolder) -> ResourceLibraryResult<()> {
        let mut connection = self.connection.lock().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(sql_error)?;
        let before = resource_paths(&transaction, folder.namespace)?;
        transaction
            .execute(
                r"
                INSERT INTO resource_library_nodes(
                    node_id, namespace, parent_folder_id, node_kind, owner_local_id,
                    identifier, display_name, created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, 'folder', NULL, ?4, ?5, ?6, ?7)
                ON CONFLICT(node_id) DO UPDATE SET
                    parent_folder_id = excluded.parent_folder_id,
                    identifier = excluded.identifier,
                    display_name = excluded.display_name,
                    updated_at_ms = excluded.updated_at_ms
                ",
                params![
                    folder.id.as_str(),
                    folder.namespace.as_str(),
                    folder.parent_id.as_ref().map(LibraryFolderId::as_str),
                    folder.identifier.as_str(),
                    folder.display_name,
                    to_i64(folder.created_at_ms)?,
                    to_i64(folder.updated_at_ms)?,
                ],
            )
            .map_err(sql_error)?;
        let after = resource_paths(&transaction, folder.namespace)?;
        apply_path_changes(
            &transaction,
            folder.namespace,
            &before,
            &after,
            folder.updated_at_ms,
        )?;
        transaction.commit().map_err(sql_error)
    }

    async fn save_resource(&self, resource: LibraryResource) -> ResourceLibraryResult<()> {
        let mut connection = self.connection.lock().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(sql_error)?;
        let before = resource_paths(&transaction, resource.namespace)?;
        transaction
            .execute(
                r"
                INSERT INTO resource_library_nodes(
                    node_id, namespace, parent_folder_id, node_kind, owner_local_id,
                    identifier, display_name, created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, 'resource', ?1, ?4, ?5, ?6, ?7)
                ON CONFLICT(node_id) DO UPDATE SET
                    parent_folder_id = excluded.parent_folder_id,
                    identifier = excluded.identifier,
                    display_name = excluded.display_name,
                    updated_at_ms = excluded.updated_at_ms
                ",
                params![
                    resource.id.as_str(),
                    resource.namespace.as_str(),
                    resource.folder_id.as_ref().map(LibraryFolderId::as_str),
                    resource.name.identifier.as_str(),
                    resource.name.display_name,
                    to_i64(resource.created_at_ms)?,
                    to_i64(resource.updated_at_ms)?,
                ],
            )
            .map_err(sql_error)?;
        replace_aliases(&transaction, &resource)?;
        let after = resource_paths(&transaction, resource.namespace)?;
        apply_path_changes(
            &transaction,
            resource.namespace,
            &before,
            &after,
            resource.updated_at_ms,
        )?;
        sync_display_name(&transaction, &resource)?;
        transaction.commit().map_err(sql_error)
    }

    async fn delete_folder(&self, id: &LibraryFolderId) -> ResourceLibraryResult<()> {
        delete_folder_cascade(&self.connection, id)
    }

    async fn delete_resource(&self, id: &LibraryResourceId) -> ResourceLibraryResult<()> {
        delete_node(&self.connection, id.as_str())
    }
}

fn load_aliases(
    connection: &rusqlite::Connection,
) -> ResourceLibraryResult<BTreeMap<String, Vec<String>>> {
    let mut statement = connection
        .prepare(
            "SELECT node_id, alias FROM resource_library_aliases ORDER BY node_id, alias_order",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sql_error)?;
    let mut aliases = BTreeMap::<String, Vec<String>>::new();
    for row in rows {
        let (node_id, alias) = row.map_err(sql_error)?;
        aliases.entry(node_id).or_default().push(alias);
    }
    Ok(aliases)
}

fn replace_aliases(
    transaction: &rusqlite::Transaction<'_>,
    resource: &LibraryResource,
) -> ResourceLibraryResult<()> {
    transaction
        .execute(
            "DELETE FROM resource_library_aliases WHERE node_id = ?1",
            [resource.id.as_str()],
        )
        .map_err(sql_error)?;
    for (index, alias) in resource.name.aliases.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO resource_library_aliases(node_id, alias_order, alias) VALUES (?1, ?2, ?3)",
                params![resource.id.as_str(), to_i64(index as u64)?, alias],
            )
            .map_err(sql_error)?;
    }
    Ok(())
}

fn delete_node(connection: &DatabaseConnection, id: &str) -> ResourceLibraryResult<()> {
    connection
        .lock()
        .map_err(repository_error)?
        .execute(
            "DELETE FROM resource_library_nodes WHERE node_id = ?1",
            [id],
        )
        .map(|_| ())
        .map_err(sql_error)
}

#[derive(Clone, Debug)]
struct DeleteResource {
    id: String,
    namespace: String,
    path: String,
}

fn delete_folder_cascade(
    connection: &DatabaseConnection,
    id: &LibraryFolderId,
) -> ResourceLibraryResult<()> {
    let mut connection = connection.lock().map_err(repository_error)?;
    let transaction = connection.transaction().map_err(sql_error)?;
    let mut statement = transaction
        .prepare(
            r"
            WITH RECURSIVE descendants(node_id, namespace, node_kind, identifier, parent_folder_id, depth) AS (
                SELECT node_id, namespace, node_kind, identifier, parent_folder_id, 0
                FROM resource_library_nodes WHERE node_id = ?1 AND node_kind = 'folder'
                UNION ALL
                SELECT child.node_id, child.namespace, child.node_kind, child.identifier,
                       child.parent_folder_id, descendants.depth + 1
                FROM resource_library_nodes child
                JOIN descendants ON child.parent_folder_id = descendants.node_id
            ), paths(node_id, path) AS (
                SELECT node_id, identifier FROM resource_library_nodes
                WHERE parent_folder_id IS NULL
                UNION ALL
                SELECT child.node_id, paths.path || '/' || child.identifier
                FROM resource_library_nodes child JOIN paths ON child.parent_folder_id = paths.node_id
            )
            SELECT descendants.node_id, descendants.namespace, paths.path, descendants.node_kind,
                   descendants.depth
            FROM descendants JOIN paths USING(node_id)
            ORDER BY descendants.depth DESC
            ",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map([id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(sql_error)?;
    let nodes = rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?;
    drop(statement);
    if nodes.is_empty() {
        return Err(ResourceLibraryError::new(
            ResourceLibraryErrorKind::NotFound,
            "library folder does not exist",
        ));
    }
    let resources = nodes
        .iter()
        .filter(|(_, _, _, kind, _)| kind == "resource")
        .map(|(id, namespace, path, _, _)| DeleteResource {
            id: id.clone(),
            namespace: namespace.clone(),
            path: path.clone(),
        })
        .collect::<Vec<_>>();
    ensure_delete_is_unreferenced(&transaction, &resources)?;
    for resource in &resources {
        delete_feature_resource(&transaction, resource)?;
    }
    for (node_id, _, _, _, _) in nodes {
        transaction
            .execute(
                "DELETE FROM resource_library_nodes WHERE node_id = ?1",
                [node_id],
            )
            .map_err(sql_error)?;
    }
    transaction.commit().map_err(sql_error)
}

fn ensure_delete_is_unreferenced(
    transaction: &rusqlite::Transaction<'_>,
    resources: &[DeleteResource],
) -> ResourceLibraryResult<()> {
    let deleted_ids = resources
        .iter()
        .map(|resource| resource.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let chunk_keys = resources
        .iter()
        .filter(|resource| resource.namespace == "prompt_chunk")
        .filter_map(|resource| PromptChunkKey::parse(&resource.path).ok())
        .collect::<Vec<_>>();
    let mut editable_texts = Vec::new();
    for (table, id_column, columns) in [
        ("prompt_chunks", "chunk_id", vec!["content"]),
        (
            "prompt_presets",
            "preset_id",
            vec![
                "before_text",
                "after_text",
                "replace_text",
                "uc_before_text",
                "uc_after_text",
                "uc_replace_text",
            ],
        ),
    ] {
        for column in columns {
            let sql = format!("SELECT {id_column}, {column} FROM {table}");
            let mut statement = transaction.prepare(&sql).map_err(sql_error)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(sql_error)?;
            editable_texts.extend(
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(sql_error)?
                    .into_iter()
                    .filter(|(owner_id, _)| !deleted_ids.contains(owner_id.as_str()))
                    .map(|(_, text)| text),
            );
        }
    }
    if chunk_keys.iter().any(|key| {
        editable_texts
            .iter()
            .any(|text| chunk_references_in_text(text, key))
    }) {
        return Err(ResourceLibraryError::new(
            ResourceLibraryErrorKind::Conflict,
            "folder contains prompt chunks referenced from outside the deleted subtree",
        ));
    }
    if let Some(draft) = transaction
        .query_row(
            "SELECT value_json FROM workspace_settings WHERE setting_key = 'generation.draft'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error)?
    {
        let value: Value = serde_json::from_str(&draft).map_err(json_error)?;
        if chunk_keys
            .iter()
            .any(|key| json_references_chunk(&value, key))
            || resources
                .iter()
                .any(|resource| json_contains_string(&value, &resource.id))
        {
            return Err(ResourceLibraryError::new(
                ResourceLibraryErrorKind::Conflict,
                "folder contains resources referenced by the current generation draft",
            ));
        }
    }
    Ok(())
}

fn delete_feature_resource(
    transaction: &rusqlite::Transaction<'_>,
    resource: &DeleteResource,
) -> ResourceLibraryResult<()> {
    match resource.namespace.as_str() {
        "prompt_chunk" => {
            transaction
                .execute(
                    "DELETE FROM prompt_chunks WHERE chunk_id = ?1",
                    [&resource.id],
                )
                .map_err(sql_error)?;
            transaction
                .execute(
                    "DELETE FROM resource_links WHERE owner_kind = 'prompt_resource' AND owner_local_id = ?1",
                    [format!("chunk:{}", resource.id)],
                )
                .map_err(sql_error)?;
        }
        "main_preset" | "character_preset" => {
            transaction
                .execute(
                    "DELETE FROM prompt_presets WHERE preset_id = ?1",
                    [&resource.id],
                )
                .map_err(sql_error)?;
            transaction
                .execute(
                    "DELETE FROM resource_links WHERE owner_kind = 'prompt_resource' AND owner_local_id = ?1",
                    [format!("preset:{}", resource.id)],
                )
                .map_err(sql_error)?;
        }
        "vibe" => {
            transaction
                .execute(
                    "DELETE FROM vibe_encodings WHERE vibe_id = ?1",
                    [&resource.id],
                )
                .map_err(sql_error)?;
            transaction
                .execute(
                    "DELETE FROM vibe_documents WHERE vibe_id = ?1",
                    [&resource.id],
                )
                .map_err(sql_error)?;
            transaction
                .execute(
                    "DELETE FROM resource_links WHERE owner_kind = 'vibe' AND owner_local_id = ?1",
                    [&resource.id],
                )
                .map_err(sql_error)?;
        }
        _ => {}
    }
    Ok(())
}

pub fn json_references_chunk(value: &Value, key: &PromptChunkKey) -> bool {
    match value {
        Value::String(text) => chunk_references_in_text(text, key),
        Value::Array(values) => values.iter().any(|value| json_references_chunk(value, key)),
        Value::Object(values) => values
            .values()
            .any(|value| json_references_chunk(value, key)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn json_contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(text) => text == needle,
        Value::Array(values) => values
            .iter()
            .any(|value| json_contains_string(value, needle)),
        Value::Object(values) => values
            .values()
            .any(|value| json_contains_string(value, needle)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn resource_paths(
    transaction: &rusqlite::Transaction<'_>,
    namespace: LibraryNamespace,
) -> ResourceLibraryResult<BTreeMap<String, String>> {
    let mut statement = transaction
        .prepare(
            r"
            WITH RECURSIVE paths(node_id, node_kind, path) AS (
                SELECT node_id, node_kind, identifier
                FROM resource_library_nodes
                WHERE namespace = ?1 AND parent_folder_id IS NULL
                UNION ALL
                SELECT child.node_id, child.node_kind, paths.path || '/' || child.identifier
                FROM resource_library_nodes child
                JOIN paths ON child.parent_folder_id = paths.node_id
                WHERE child.namespace = ?1
            )
            SELECT node_id, path FROM paths WHERE node_kind = 'resource'
            ",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map([namespace.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(sql_error)?;
    rows.collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(sql_error)
}

fn apply_path_changes(
    transaction: &rusqlite::Transaction<'_>,
    namespace: LibraryNamespace,
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    updated_at_ms: u64,
) -> ResourceLibraryResult<()> {
    let changes = before
        .iter()
        .filter_map(|(id, old_path)| {
            after
                .get(id)
                .filter(|new_path| *new_path != old_path)
                .map(|new_path| (id.as_str(), old_path.as_str(), new_path.as_str()))
        })
        .collect::<Vec<_>>();
    if changes.is_empty() {
        return Ok(());
    }
    let updated = to_i64(updated_at_ms)?;
    match namespace {
        LibraryNamespace::PromptChunk => {
            for (id, _, new_path) in &changes {
                transaction
                    .execute(
                        "UPDATE prompt_chunks SET chunk_key = ?2, updated_at_ms = ?3 WHERE chunk_id = ?1",
                        params![id, new_path, updated],
                    )
                    .map_err(sql_error)?;
            }
            rewrite_editable_chunk_references(transaction, &changes, updated)?;
        }
        LibraryNamespace::MainPreset | LibraryNamespace::CharacterPreset => {
            for (id, _, new_path) in changes {
                transaction
                    .execute(
                        "UPDATE prompt_presets SET category = ?2, updated_at_ms = ?3 WHERE preset_id = ?1",
                        params![id, new_path, updated],
                    )
                    .map_err(sql_error)?;
            }
        }
        LibraryNamespace::Vibe => {}
    }
    Ok(())
}

fn rewrite_editable_chunk_references(
    transaction: &rusqlite::Transaction<'_>,
    changes: &[(&str, &str, &str)],
    updated_at_ms: i64,
) -> ResourceLibraryResult<()> {
    let mappings = changes
        .iter()
        .map(|(_, old, new)| {
            Ok((
                PromptChunkKey::parse(old).map_err(prompt_library_error)?,
                PromptChunkKey::parse(new).map_err(prompt_library_error)?,
            ))
        })
        .collect::<ResourceLibraryResult<Vec<_>>>()?;
    rewrite_table_texts(
        transaction,
        "prompt_chunks",
        "chunk_id",
        &["content"],
        &mappings,
        updated_at_ms,
    )?;
    rewrite_table_texts(
        transaction,
        "prompt_presets",
        "preset_id",
        &[
            "before_text",
            "after_text",
            "replace_text",
            "uc_before_text",
            "uc_after_text",
            "uc_replace_text",
        ],
        &mappings,
        updated_at_ms,
    )?;
    rewrite_generation_draft(transaction, &mappings)?;
    Ok(())
}

fn rewrite_table_texts(
    transaction: &rusqlite::Transaction<'_>,
    table: &str,
    id_column: &str,
    columns: &[&str],
    mappings: &[(PromptChunkKey, PromptChunkKey)],
    updated_at_ms: i64,
) -> ResourceLibraryResult<()> {
    for column in columns {
        let select = format!("SELECT {id_column}, {column} FROM {table}");
        let mut statement = transaction.prepare(&select).map_err(sql_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(sql_error)?;
        let values = rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?;
        drop(statement);
        for (id, text) in values {
            let rewritten = mappings.iter().fold(text.clone(), |current, (old, new)| {
                rewrite_chunk_references(&current, old, new)
            });
            if rewritten != text {
                let update = format!(
                    "UPDATE {table} SET {column} = ?2, updated_at_ms = ?3 WHERE {id_column} = ?1"
                );
                transaction
                    .execute(&update, params![id, rewritten, updated_at_ms])
                    .map_err(sql_error)?;
            }
        }
    }
    Ok(())
}

pub fn rewrite_generation_draft(
    transaction: &rusqlite::Transaction<'_>,
    mappings: &[(PromptChunkKey, PromptChunkKey)],
) -> ResourceLibraryResult<()> {
    let Some(mut current) =
        crate::generation_draft::load_on_connection(transaction).map_err(|error| {
            ResourceLibraryError::new(ResourceLibraryErrorKind::Repository, error.to_string())
        })?
    else {
        return Ok(());
    };
    atelier_prompt_resources::rewrite_draft_chunk_references(&mut current.snapshot, mappings);
    crate::generation_draft::save_on_connection(transaction, current.revision, &current.snapshot)
        .map(|_| ())
        .map_err(|error| {
            ResourceLibraryError::new(ResourceLibraryErrorKind::Repository, error.to_string())
        })
}

fn sync_display_name(
    transaction: &rusqlite::Transaction<'_>,
    resource: &LibraryResource,
) -> ResourceLibraryResult<()> {
    match resource.namespace {
        LibraryNamespace::PromptChunk => Ok(()),
        LibraryNamespace::MainPreset | LibraryNamespace::CharacterPreset => transaction
            .execute(
                "UPDATE prompt_presets SET name = ?2 WHERE preset_id = ?1",
                params![resource.id.as_str(), resource.name.display_name],
            )
            .map(|_| ())
            .map_err(sql_error),
        LibraryNamespace::Vibe => sync_vibe_display(transaction, resource),
    }
}

fn sync_vibe_display(
    transaction: &rusqlite::Transaction<'_>,
    resource: &LibraryResource,
) -> ResourceLibraryResult<()> {
    let json = transaction
        .query_row(
            "SELECT document_json FROM vibe_documents WHERE vibe_id = ?1",
            [resource.id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sql_error)?;
    let Some(json) = json else {
        return Ok(());
    };
    let mut value: Value = serde_json::from_str(&json).map_err(json_error)?;
    if let Some(summary) = value.get_mut("summary").and_then(Value::as_object_mut) {
        summary.insert(
            "display_name".to_owned(),
            Value::String(resource.name.display_name.clone()),
        );
    }
    transaction
        .execute(
            "UPDATE vibe_documents SET display_name = ?2, document_json = ?3 WHERE vibe_id = ?1",
            params![
                resource.id.as_str(),
                resource.name.display_name,
                serde_json::to_string(&value).map_err(json_error)?,
            ],
        )
        .map(|_| ())
        .map_err(sql_error)
}

fn prompt_library_error(
    error: atelier_prompt_resources::PromptResourceError,
) -> ResourceLibraryError {
    ResourceLibraryError::new(ResourceLibraryErrorKind::Repository, error.to_string())
}

fn json_error(error: serde_json::Error) -> ResourceLibraryError {
    ResourceLibraryError::new(ResourceLibraryErrorKind::Repository, error.to_string())
}

fn namespace_from_str(value: &str) -> ResourceLibraryResult<LibraryNamespace> {
    match value {
        "prompt_chunk" => Ok(LibraryNamespace::PromptChunk),
        "main_preset" => Ok(LibraryNamespace::MainPreset),
        "character_preset" => Ok(LibraryNamespace::CharacterPreset),
        "vibe" => Ok(LibraryNamespace::Vibe),
        _ => Err(ResourceLibraryError::new(
            ResourceLibraryErrorKind::Repository,
            format!("unknown resource library namespace `{value}`"),
        )),
    }
}

fn to_i64(value: u64) -> ResourceLibraryResult<i64> {
    i64::try_from(value).map_err(|error| repository_error(DatabaseError::new(error.to_string())))
}

fn to_u64(value: i64) -> ResourceLibraryResult<u64> {
    u64::try_from(value).map_err(|error| repository_error(DatabaseError::new(error.to_string())))
}

fn sql_error(error: rusqlite::Error) -> ResourceLibraryError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(_, Some(ref message)) if message.contains("UNIQUE constraint failed")
    ) {
        return ResourceLibraryError::new(
            ResourceLibraryErrorKind::Conflict,
            "resource library identifier already exists in this folder",
        );
    }
    repository_error(DatabaseError::from(error))
}

fn repository_error(error: DatabaseError) -> ResourceLibraryError {
    ResourceLibraryError::new(ResourceLibraryErrorKind::Repository, error.to_string())
}
