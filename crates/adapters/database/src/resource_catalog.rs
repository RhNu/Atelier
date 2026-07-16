#![allow(clippy::significant_drop_tightening)]

use std::collections::HashSet;

use async_trait::async_trait;
use atelier_artifacts::{ArtifactError, ArtifactResourceReader, ArtifactResult};
use atelier_resource_catalog::{
    BlobId, ResourceCatalogError, ResourceCatalogRepository, ResourceCatalogTransaction,
    ResourceCleanupCandidate, ResourceId, ResourceLink, ResourceMetadata, ResourceOwner,
    ResourceRecord, ResourceRef, ResourceResult, ResourceState, ResourceVariant, VariantId,
};
use rusqlite::{OptionalExtension, Row, params};

use crate::codec::{
    lifecycle_as_str, lifecycle_from_str, metadata_from_columns, owner_kind_as_str,
    owner_kind_from_str, relation_as_str, relation_from_str, resource_kind_as_str,
    resource_kind_from_str, resource_state_as_str, resource_state_from_str, variant_kind_as_str,
    variant_kind_from_str,
};
use crate::connection::{DatabaseConnection, DatabaseTransactionGate};
use crate::error::DatabaseError;

#[derive(Clone, Debug)]
pub struct DatabaseResourceCatalogRepository {
    connection: DatabaseConnection,
}

impl DatabaseResourceCatalogRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl ResourceCatalogRepository for DatabaseResourceCatalogRepository {
    async fn begin_transaction(&self) -> ResourceResult<Box<dyn ResourceCatalogTransaction>> {
        Ok(Box::new(DatabaseResourceCatalogTransaction {
            connection: self.connection.clone(),
            _gate: self
                .connection
                .acquire_transaction_gate()
                .await
                .map_err(resource_error)?,
            pending_records: Vec::new(),
            links_to_attach: HashSet::new(),
            links_to_detach: HashSet::new(),
            variants: Vec::new(),
            ready_records: HashSet::new(),
            delete_pending: HashSet::new(),
            orphan_markers_to_clear: HashSet::new(),
        }))
    }

    async fn get_ready_record(&self, id: &ResourceId) -> ResourceResult<Option<ResourceRecord>> {
        let connection = self.connection.lock().map_err(resource_error)?;
        connection
            .query_row(
                resource_select_sql("WHERE id = ?1 AND state = 'ready'").as_str(),
                params![id.as_str()],
                resource_record_from_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn list_ready_refs_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceRef>> {
        Ok(self
            .list_ready_links_by_owner(owner)
            .await?
            .into_iter()
            .map(|link| ResourceRef::base(link.resource_id))
            .collect())
    }

    async fn list_ready_links_by_owner(
        &self,
        owner: &ResourceOwner,
    ) -> ResourceResult<Vec<ResourceLink>> {
        let connection = self.connection.lock().map_err(resource_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT l.resource_id, l.owner_kind, l.owner_local_id, l.relation
                FROM resource_links l
                JOIN resources r ON r.id = l.resource_id
                WHERE l.owner_kind = ?1 AND l.owner_local_id = ?2 AND r.state = 'ready'
                ORDER BY l.resource_id ASC, l.relation ASC
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(
                params![owner_kind_as_str(owner.kind), owner.local_id.as_str()],
                resource_link_from_row,
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }

    async fn get_variant(&self, id: &VariantId) -> ResourceResult<Option<ResourceVariant>> {
        let connection = self.connection.lock().map_err(resource_error)?;
        connection
            .query_row(
                r"
                SELECT variant_id, resource_id, kind, blob_id, mime_type, byte_size,
                       content_hash, width, height, created_at_ms
                FROM resource_variants
                WHERE variant_id = ?1
                ",
                params![id.as_str()],
                resource_variant_from_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn list_delete_pending_resources(&self) -> ResourceResult<Vec<ResourceCleanupCandidate>> {
        let connection = self.connection.lock().map_err(resource_error)?;
        let mut statement = connection
            .prepare(resource_select_sql("WHERE state = 'delete_pending' ORDER BY id ASC").as_str())
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], resource_record_from_row)
            .map_err(sql_error)?;
        let records = rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)?;
        records
            .into_iter()
            .map(|record| {
                let variants = list_variants_for_resource(&connection, &record.id)?;
                Ok(ResourceCleanupCandidate { record, variants })
            })
            .collect()
    }

    async fn blob_is_referenced_outside_resource(
        &self,
        resource_id: &ResourceId,
        blob_id: &BlobId,
    ) -> ResourceResult<bool> {
        let connection = self.connection.lock().map_err(resource_error)?;
        connection
            .query_row(
                r"
                SELECT EXISTS(
                    SELECT 1 FROM resources
                    WHERE blob_id = ?2 AND id <> ?1
                    UNION ALL
                    SELECT 1 FROM resource_variants
                    WHERE blob_id = ?2 AND resource_id <> ?1
                )
                ",
                params![resource_id.as_str(), blob_id.as_str()],
                |row| row.get::<_, bool>(0),
            )
            .map_err(sql_error)
    }

    async fn delete_resource_record_if_unowned(&self, id: &ResourceId) -> ResourceResult<bool> {
        {
            let mut connection = self.connection.lock().map_err(resource_error)?;
            let tx = connection.transaction().map_err(sql_error)?;
            let deleted = tx
                .execute(
                    r"
                    DELETE FROM resources
                    WHERE id = ?1
                      AND state = 'delete_pending'
                      AND NOT EXISTS (
                          SELECT 1 FROM resource_links WHERE resource_id = resources.id
                      )
                    ",
                    params![id.as_str()],
                )
                .map_err(sql_error)?;
            tx.commit().map_err(sql_error)?;
            drop(connection);
            return Ok(deleted == 1);
        }
    }

    async fn blob_is_referenced(&self, blob_id: &BlobId) -> ResourceResult<bool> {
        let connection = self.connection.lock().map_err(resource_error)?;
        connection
            .query_row(
                r"
                SELECT EXISTS(
                    SELECT 1 FROM resources WHERE blob_id = ?1
                    UNION ALL
                    SELECT 1 FROM resource_variants WHERE blob_id = ?1
                )
                ",
                params![blob_id.as_str()],
                |row| row.get::<_, bool>(0),
            )
            .map_err(sql_error)
    }

    async fn scan_orphan_blobs(&self) -> ResourceResult<Vec<BlobId>> {
        let connection = self.connection.lock().map_err(resource_error)?;
        let mut statement = connection
            .prepare("SELECT blob_id FROM orphan_blobs ORDER BY blob_id ASC")
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], |row| Ok(BlobId::new(row.get::<_, String>(0)?)))
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }

    async fn record_orphan_blob(&self, blob_id: &BlobId) -> ResourceResult<()> {
        let connection = self.connection.lock().map_err(resource_error)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO orphan_blobs(blob_id) VALUES (?1)",
                params![blob_id.as_str()],
            )
            .map(|_| ())
            .map_err(sql_error)
    }
}

#[async_trait]
impl ArtifactResourceReader for DatabaseResourceCatalogRepository {
    async fn get_artifact_resource(
        &self,
        reference: &ResourceRef,
    ) -> ArtifactResult<ResourceRecord> {
        if let Some(variant_id) = &reference.variant_id {
            let variant = self
                .get_variant(variant_id)
                .await
                .map_err(|error| ArtifactError::resource(error.to_string()))?
                .ok_or_else(|| {
                    ArtifactError::resource("artifact resource variant does not exist")
                })?;
            if variant.resource_id != reference.id {
                return Err(ArtifactError::resource(
                    "artifact resource variant belongs to another resource",
                ));
            }
        }

        self.get_ready_record(&reference.id)
            .await
            .map_err(|error| ArtifactError::resource(error.to_string()))?
            .ok_or_else(|| ArtifactError::resource("artifact resource does not exist"))
    }
}

struct DatabaseResourceCatalogTransaction {
    connection: DatabaseConnection,
    _gate: DatabaseTransactionGate,
    pending_records: Vec<ResourceRecord>,
    links_to_attach: HashSet<ResourceLink>,
    links_to_detach: HashSet<ResourceLink>,
    variants: Vec<ResourceVariant>,
    ready_records: HashSet<ResourceId>,
    delete_pending: HashSet<ResourceId>,
    orphan_markers_to_clear: HashSet<BlobId>,
}

#[async_trait]
impl ResourceCatalogTransaction for DatabaseResourceCatalogTransaction {
    async fn insert_pending_record(&mut self, record: ResourceRecord) -> ResourceResult<()> {
        self.pending_records.push(record);
        Ok(())
    }

    async fn attach_owner(&mut self, link: ResourceLink) -> ResourceResult<()> {
        self.links_to_attach.insert(link);
        Ok(())
    }

    async fn detach_owner(&mut self, link: &ResourceLink) -> ResourceResult<()> {
        self.links_to_detach.insert(link.clone());
        Ok(())
    }

    async fn count_owner_links(&self, id: &ResourceId) -> ResourceResult<usize> {
        let connection = self.connection.lock().map_err(resource_error)?;
        let mut statement = connection
            .prepare(
                "SELECT resource_id, owner_kind, owner_local_id, relation FROM resource_links WHERE resource_id = ?1",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(params![id.as_str()], resource_link_from_row)
            .map_err(sql_error)?;
        let persisted = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_error)?
            .into_iter()
            .filter(|link| !self.links_to_detach.contains(link))
            .count();
        let attached = self
            .links_to_attach
            .iter()
            .filter(|link| &link.resource_id == id)
            .count();
        Ok(persisted + attached)
    }

    async fn mark_ready(&mut self, id: &ResourceId) -> ResourceResult<()> {
        self.ready_records.insert(id.clone());
        Ok(())
    }

    async fn mark_delete_pending(&mut self, id: &ResourceId) -> ResourceResult<()> {
        self.delete_pending.insert(id.clone());
        Ok(())
    }

    async fn insert_variant(&mut self, variant: ResourceVariant) -> ResourceResult<()> {
        self.variants.push(variant);
        Ok(())
    }

    async fn clear_orphan_blob_marker(&mut self, blob_id: &BlobId) -> ResourceResult<()> {
        self.orphan_markers_to_clear.insert(blob_id.clone());
        Ok(())
    }

    async fn commit(self: Box<Self>) -> ResourceResult<()> {
        let mut connection = self.connection.lock().map_err(resource_error)?;
        let tx = connection.transaction().map_err(sql_error)?;
        for mut record in self.pending_records {
            if self.ready_records.contains(&record.id) {
                record.state = ResourceState::Ready;
            }
            insert_resource_record(&tx, &record)?;
        }
        for link in &self.links_to_detach {
            tx.execute(
                r"
                DELETE FROM resource_links
                WHERE resource_id = ?1 AND owner_kind = ?2 AND owner_local_id = ?3 AND relation = ?4
                ",
                params![
                    link.resource_id.as_str(),
                    owner_kind_as_str(link.owner.kind),
                    link.owner.local_id.as_str(),
                    relation_as_str(link.relation),
                ],
            )
            .map_err(sql_error)?;
        }
        for link in &self.links_to_attach {
            tx.execute(
                r"
                INSERT OR IGNORE INTO resource_links(resource_id, owner_kind, owner_local_id, relation)
                VALUES (?1, ?2, ?3, ?4)
                ",
                params![
                    link.resource_id.as_str(),
                    owner_kind_as_str(link.owner.kind),
                    link.owner.local_id.as_str(),
                    relation_as_str(link.relation),
                ],
            )
            .map_err(sql_error)?;
        }
        for id in &self.delete_pending {
            tx.execute(
                "UPDATE resources SET state = 'delete_pending' WHERE id = ?1",
                params![id.as_str()],
            )
            .map_err(sql_error)?;
        }
        for variant in &self.variants {
            insert_resource_variant(&tx, variant)?;
        }
        for blob_id in &self.orphan_markers_to_clear {
            tx.execute(
                "DELETE FROM orphan_blobs WHERE blob_id = ?1",
                params![blob_id.as_str()],
            )
            .map_err(sql_error)?;
        }
        tx.commit().map_err(sql_error)
    }

    async fn rollback(self: Box<Self>) -> ResourceResult<()> {
        Ok(())
    }
}

fn insert_resource_record(
    connection: &rusqlite::Transaction<'_>,
    record: &ResourceRecord,
) -> ResourceResult<()> {
    connection
        .execute(
            r"
            INSERT INTO resources(
                id, kind, lifecycle, state, blob_id, mime_type, byte_size,
                content_hash, width, height, created_at_ms
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ",
            params![
                record.id.as_str(),
                resource_kind_as_str(record.kind),
                lifecycle_as_str(record.lifecycle),
                resource_state_as_str(record.state),
                record.blob_id.as_str(),
                record.metadata.mime_type.as_deref(),
                optional_u64_to_i64(record.metadata.byte_size)?,
                record.metadata.content_hash.as_deref(),
                optional_u32_to_i64(record.metadata.width),
                optional_u32_to_i64(record.metadata.height),
                optional_u64_to_i64(record.metadata.created_at_ms)?,
            ],
        )
        .map(|_| ())
        .map_err(sql_error)
}

fn insert_resource_variant(
    connection: &rusqlite::Transaction<'_>,
    variant: &ResourceVariant,
) -> ResourceResult<()> {
    connection
        .execute(
            r"
            INSERT INTO resource_variants(
                variant_id, resource_id, kind, blob_id, mime_type, byte_size,
                content_hash, width, height, created_at_ms
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ",
            params![
                variant.id.as_str(),
                variant.resource_id.as_str(),
                variant_kind_as_str(variant.kind),
                variant.blob_id.as_str(),
                variant.metadata.mime_type.as_deref(),
                optional_u64_to_i64(variant.metadata.byte_size)?,
                variant.metadata.content_hash.as_deref(),
                optional_u32_to_i64(variant.metadata.width),
                optional_u32_to_i64(variant.metadata.height),
                optional_u64_to_i64(variant.metadata.created_at_ms)?,
            ],
        )
        .map(|_| ())
        .map_err(sql_error)
}

fn resource_select_sql(where_clause: &str) -> String {
    format!(
        "SELECT id, kind, lifecycle, state, blob_id, mime_type, byte_size, content_hash, width, height, created_at_ms FROM resources {where_clause}"
    )
}

fn resource_record_from_row(row: &Row<'_>) -> rusqlite::Result<ResourceRecord> {
    let kind = row.get::<_, String>(1)?;
    let lifecycle = row.get::<_, String>(2)?;
    let state = row.get::<_, String>(3)?;
    Ok(ResourceRecord {
        id: ResourceId::new(row.get::<_, String>(0)?),
        kind: resource_kind_from_str(&kind).map_err(to_sql_error)?,
        lifecycle: lifecycle_from_str(&lifecycle).map_err(to_sql_error)?,
        state: resource_state_from_str(&state).map_err(to_sql_error)?,
        blob_id: BlobId::new(row.get::<_, String>(4)?),
        metadata: metadata_from_row(row, 5)?,
    })
}

fn resource_link_from_row(row: &Row<'_>) -> rusqlite::Result<ResourceLink> {
    let owner_kind = row.get::<_, String>(1)?;
    let relation = row.get::<_, String>(3)?;
    Ok(ResourceLink::new(
        ResourceId::new(row.get::<_, String>(0)?),
        ResourceOwner {
            kind: owner_kind_from_str(&owner_kind).map_err(to_sql_error)?,
            local_id: row.get::<_, String>(2)?,
        },
        relation_from_str(&relation).map_err(to_sql_error)?,
    ))
}

fn list_variants_for_resource(
    connection: &rusqlite::Connection,
    resource_id: &ResourceId,
) -> ResourceResult<Vec<ResourceVariant>> {
    let mut statement = connection
        .prepare(
            r"
            SELECT variant_id, resource_id, kind, blob_id, mime_type, byte_size,
                   content_hash, width, height, created_at_ms
            FROM resource_variants
            WHERE resource_id = ?1
            ORDER BY variant_id ASC
            ",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map(params![resource_id.as_str()], resource_variant_from_row)
        .map_err(sql_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn resource_variant_from_row(row: &Row<'_>) -> rusqlite::Result<ResourceVariant> {
    let kind = row.get::<_, String>(2)?;
    Ok(ResourceVariant {
        id: VariantId::new(row.get::<_, String>(0)?),
        resource_id: ResourceId::new(row.get::<_, String>(1)?),
        kind: variant_kind_from_str(&kind).map_err(to_sql_error)?,
        blob_id: BlobId::new(row.get::<_, String>(3)?),
        metadata: metadata_from_row(row, 4)?,
    })
}

fn metadata_from_row(row: &Row<'_>, start: usize) -> rusqlite::Result<ResourceMetadata> {
    Ok(metadata_from_columns(
        row.get(start)?,
        optional_i64_to_u64(row.get(start + 1)?)?,
        row.get(start + 2)?,
        optional_i64_to_u32(row.get(start + 3)?)?,
        optional_i64_to_u32(row.get(start + 4)?)?,
        optional_i64_to_u64(row.get(start + 5)?)?,
    ))
}

fn optional_u64_to_i64(value: Option<u64>) -> ResourceResult<Option<i64>> {
    value
        .map(|item| {
            i64::try_from(item).map_err(|error| ResourceCatalogError::repository(error.to_string()))
        })
        .transpose()
}

fn optional_u32_to_i64(value: Option<u32>) -> Option<i64> {
    value.map(i64::from)
}

fn optional_i64_to_u64(value: Option<i64>) -> rusqlite::Result<Option<u64>> {
    value
        .map(|item| {
            u64::try_from(item).map_err(|error| to_sql_error(DatabaseError::new(error.to_string())))
        })
        .transpose()
}

fn optional_i64_to_u32(value: Option<i64>) -> rusqlite::Result<Option<u32>> {
    value
        .map(|item| {
            u32::try_from(item).map_err(|error| to_sql_error(DatabaseError::new(error.to_string())))
        })
        .transpose()
}

fn sql_error(error: rusqlite::Error) -> ResourceCatalogError {
    let message = error.to_string();
    drop(error);
    ResourceCatalogError::repository(message)
}

fn resource_error(error: DatabaseError) -> ResourceCatalogError {
    let message = error.to_string();
    drop(error);
    ResourceCatalogError::repository(message)
}

fn to_sql_error(error: DatabaseError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
