use std::collections::BTreeMap;

use async_trait::async_trait;
use atelier_resource_library::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    ResourceIdentifier, ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryRepository,
    ResourceLibraryResult, ResourceName,
};
use rusqlite::{OptionalExtension, params};

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

    pub async fn get_resource_by_owner(
        &self,
        namespace: LibraryNamespace,
        owner_local_id: &str,
    ) -> ResourceLibraryResult<Option<LibraryResource>> {
        let resources = self.load_resources(Some(namespace))?;
        let connection = self.connection.lock().map_err(repository_error)?;
        let node_id = connection
            .query_row(
                "SELECT node_id FROM resource_library_nodes WHERE namespace = ?1 AND owner_local_id = ?2",
                params![namespace.as_str(), owner_local_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?;
        Ok(node_id.and_then(|id| {
            resources
                .into_iter()
                .find(|resource| resource.id.as_str() == id)
        }))
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
        let (sql, namespace_text) = namespace.map_or(
            (
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
        let connection = self.connection.lock().map_err(repository_error)?;
        connection
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
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn save_resource(&self, resource: LibraryResource) -> ResourceLibraryResult<()> {
        let mut connection = self.connection.lock().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(sql_error)?;
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
        transaction.commit().map_err(sql_error)
    }

    async fn delete_folder(&self, id: &LibraryFolderId) -> ResourceLibraryResult<()> {
        delete_node(&self.connection, id.as_str())
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
