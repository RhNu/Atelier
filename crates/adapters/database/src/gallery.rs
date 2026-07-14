#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use atelier_gallery::{
    GalleryError, GalleryIndex, GalleryItem, GalleryItemId, GalleryQuery, GalleryResult,
    GallerySafetyOverride,
};
use atelier_safety::SafetyLabel;
use rusqlite::{OptionalExtension, params};

use crate::codec::{
    GalleryItemDto, JsonCodec, artifact_kind_as_str, safety_override_as_str,
    safety_override_from_str, source_kind_as_str,
};
use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;

#[derive(Clone, Debug)]
pub struct DatabaseGalleryIndex {
    connection: DatabaseConnection,
}

#[derive(Clone, Debug)]
pub struct GalleryHardDeletePlan {
    pub item_id: String,
    pub artifact_id: String,
    pub resource_ids: Vec<String>,
    pub transient_owner: GalleryTransientOwner,
    pub force_delete_pending: bool,
}

#[derive(Clone, Debug)]
pub struct GalleryTransientOwner {
    pub kind: &'static str,
    pub local_id: String,
}

impl DatabaseGalleryIndex {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    /// Removes gallery rows and their database-owned dependencies in one transaction.
    ///
    /// # Errors
    /// Returns an error when the transaction cannot acquire the database gate or commit.
    pub async fn hard_delete(&self, plans: &[GalleryHardDeletePlan]) -> GalleryResult<usize> {
        if plans.is_empty() {
            return Ok(0);
        }
        let _gate = self
            .connection
            .acquire_transaction_gate()
            .await
            .map_err(gallery_error)?;
        let mut connection = self.connection.lock().map_err(gallery_error)?;
        let transaction = connection.transaction().map_err(sql_error)?;
        let mut deleted = 0;
        for plan in plans {
            for resource_id in &plan.resource_ids {
                transaction
                    .execute(
                        "DELETE FROM resource_links WHERE resource_id = ?1 AND owner_kind = 'gallery_item' AND owner_local_id = ?2 AND relation = 'primary'",
                        params![resource_id, plan.item_id],
                    )
                    .map_err(sql_error)?;
                transaction
                    .execute(
                        "DELETE FROM resource_links WHERE resource_id = ?1 AND owner_kind = ?2 AND owner_local_id = ?3 AND relation = 'primary'",
                        params![
                            resource_id,
                            plan.transient_owner.kind,
                            plan.transient_owner.local_id
                        ],
                    )
                    .map_err(sql_error)?;
                transaction
                    .execute(
                        r"
                        UPDATE resources SET state = 'delete_pending'
                        WHERE id = ?1
                          AND NOT EXISTS (
                              SELECT 1 FROM resource_links WHERE resource_id = ?1
                          )
                          AND (?2 = 1 OR lifecycle IN ('job_scoped', 'cache'))
                        ",
                        params![resource_id, i64::from(plan.force_delete_pending)],
                    )
                    .map_err(sql_error)?;
            }
            transaction
                .execute(
                    "DELETE FROM run_outputs WHERE item_id = ?1",
                    params![plan.item_id],
                )
                .map_err(sql_error)?;
            transaction
                .execute(
                    "DELETE FROM artifacts WHERE artifact_id = ?1",
                    params![plan.artifact_id],
                )
                .map_err(sql_error)?;
            deleted += transaction
                .execute(
                    "DELETE FROM gallery_items WHERE item_id = ?1",
                    params![plan.item_id],
                )
                .map_err(sql_error)?;
        }
        transaction.commit().map_err(sql_error)?;
        Ok(deleted)
    }
}

#[async_trait]
impl GalleryIndex for DatabaseGalleryIndex {
    async fn upsert_item(&self, item: GalleryItem) -> GalleryResult<()> {
        let json = GalleryItemDto::encode_domain(&item).map_err(gallery_error)?;
        let connection = self.connection.lock().map_err(gallery_error)?;
        connection
            .execute(
                r"
                INSERT INTO gallery_items(
                    item_id, artifact_id, artifact_kind, source_kind,
                    manual_safety_override, effective_safety_label, indexed_at_ms, item_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(item_id) DO UPDATE
                SET artifact_id = excluded.artifact_id,
                    artifact_kind = excluded.artifact_kind,
                    source_kind = excluded.source_kind,
                    manual_safety_override = excluded.manual_safety_override,
                    effective_safety_label = excluded.effective_safety_label,
                    indexed_at_ms = excluded.indexed_at_ms,
                    item_json = excluded.item_json
                ",
                params![
                    item.id.as_str(),
                    item.artifact_id.as_str(),
                    artifact_kind_as_str(item.artifact_kind),
                    source_kind_as_str(item.source_kind()),
                    item.manual_safety_override.map(safety_override_as_str),
                    item.effective_safety_label().map(safety_label_as_str),
                    i64::try_from(item.indexed_at_ms)
                        .map_err(|error| GalleryError::repository(error.to_string()))?,
                    json,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn get_item(&self, id: &GalleryItemId) -> GalleryResult<Option<GalleryItem>> {
        let connection = self.connection.lock().map_err(gallery_error)?;
        connection
            .query_row(
                "SELECT item_json, manual_safety_override FROM gallery_items WHERE item_id = ?1",
                params![id.as_str()],
                gallery_item_from_row,
            )
            .optional()
            .map_err(sql_error)
    }

    async fn query_items(&self, query: GalleryQuery) -> GalleryResult<Vec<GalleryItem>> {
        let connection = self.connection.lock().map_err(gallery_error)?;
        let artifact_kind = query.artifact_kind.map(artifact_kind_as_str);
        let source_kind = query.source_kind.map(source_kind_as_str);
        let safety_override = query.manual_safety_override.map(safety_override_as_str);
        let exclude_hidden = query.safety_label != Some(SafetyLabel::Hidden);
        let safety_label = query.safety_label.map(safety_label_as_str);
        let limit = i64::try_from(query.limit)
            .map_err(|error| GalleryError::repository(error.to_string()))?;
        let offset = i64::try_from(query.offset)
            .map_err(|error| GalleryError::repository(error.to_string()))?;
        let mut statement = connection
            .prepare(
                r"
                SELECT item_json, manual_safety_override
                FROM gallery_items
                WHERE (?1 IS NULL OR artifact_kind = ?1)
                  AND (?2 IS NULL OR source_kind = ?2)
                  AND (
                      (?3 IS NULL AND (
                          ?4 = 0
                          OR manual_safety_override IS NULL
                          OR manual_safety_override <> 'hidden'
                      ))
                      OR (?3 IS NOT NULL AND manual_safety_override = ?3)
                  )
                  AND (?5 IS NULL OR effective_safety_label = ?5)
                ORDER BY indexed_at_ms DESC, item_id ASC
                LIMIT ?6 OFFSET ?7
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(
                params![
                    artifact_kind,
                    source_kind,
                    safety_override,
                    i64::from(exclude_hidden),
                    safety_label,
                    limit,
                    offset,
                ],
                gallery_item_from_row,
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
    }

    async fn count_items(&self, query: GalleryQuery) -> GalleryResult<usize> {
        let connection = self.connection.lock().map_err(gallery_error)?;
        let artifact_kind = query.artifact_kind.map(artifact_kind_as_str);
        let source_kind = query.source_kind.map(source_kind_as_str);
        let safety_override = query.manual_safety_override.map(safety_override_as_str);
        let exclude_hidden = query.safety_label != Some(SafetyLabel::Hidden);
        let safety_label = query.safety_label.map(safety_label_as_str);
        let count = connection
            .query_row(
                r"
                SELECT COUNT(*)
                FROM gallery_items
                WHERE (?1 IS NULL OR artifact_kind = ?1)
                  AND (?2 IS NULL OR source_kind = ?2)
                  AND (
                      (?3 IS NULL AND (
                          ?4 = 0
                          OR manual_safety_override IS NULL
                          OR manual_safety_override <> 'hidden'
                      ))
                      OR (?3 IS NOT NULL AND manual_safety_override = ?3)
                  )
                  AND (?5 IS NULL OR effective_safety_label = ?5)
                ",
                params![
                    artifact_kind,
                    source_kind,
                    safety_override,
                    i64::from(exclude_hidden),
                    safety_label
                ],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sql_error)?;
        usize::try_from(count).map_err(|error| GalleryError::repository(error.to_string()))
    }

    async fn delete_items(&self, ids: &[GalleryItemId]) -> GalleryResult<Vec<GalleryItem>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let deleted = {
            let mut connection = self.connection.lock().map_err(gallery_error)?;
            let transaction = connection.transaction().map_err(sql_error)?;
            let mut deleted = Vec::new();
            for id in ids {
                let item = transaction
                    .query_row(
                        "SELECT item_json, manual_safety_override FROM gallery_items WHERE item_id = ?1",
                        params![id.as_str()],
                        gallery_item_from_row,
                    )
                    .optional()
                    .map_err(sql_error)?;
                if let Some(item) = item {
                    transaction
                        .execute(
                            "DELETE FROM gallery_items WHERE item_id = ?1",
                            params![id.as_str()],
                        )
                        .map_err(sql_error)?;
                    deleted.push(item);
                }
            }
            transaction.commit().map_err(sql_error)?;
            drop(connection);
            deleted
        };
        Ok(deleted)
    }

    async fn set_safety_override(
        &self,
        id: &GalleryItemId,
        manual_safety_override: Option<GallerySafetyOverride>,
    ) -> GalleryResult<GalleryItem> {
        let mut item = self
            .get_item(id)
            .await?
            .ok_or_else(|| GalleryError::not_found("gallery item does not exist"))?;
        item.manual_safety_override = manual_safety_override;
        let json = GalleryItemDto::encode_domain(&item).map_err(gallery_error)?;
        let connection = self.connection.lock().map_err(gallery_error)?;
        connection
            .execute(
                r"
                UPDATE gallery_items
                SET manual_safety_override = ?2,
                    effective_safety_label = ?3,
                    item_json = ?4
                WHERE item_id = ?1
                ",
                params![
                    id.as_str(),
                    manual_safety_override.map(safety_override_as_str),
                    item.effective_safety_label().map(safety_label_as_str),
                    json,
                ],
            )
            .map_err(sql_error)?;
        Ok(item)
    }
}

const fn safety_label_as_str(value: SafetyLabel) -> &'static str {
    match value {
        SafetyLabel::Safe => "safe",
        SafetyLabel::Sensitive => "sensitive",
        SafetyLabel::Hidden => "hidden",
    }
}

fn gallery_item_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<GalleryItem> {
    let json = row.get::<_, String>(0)?;
    let manual = row.get::<_, Option<String>>(1)?;
    let mut item = GalleryItemDto::decode_domain(&json).map_err(to_sql_error)?;
    item.manual_safety_override = manual
        .as_deref()
        .map(safety_override_from_str)
        .transpose()
        .map_err(to_sql_error)?;
    Ok(item)
}

fn sql_error(error: rusqlite::Error) -> GalleryError {
    let message = error.to_string();
    drop(error);
    GalleryError::repository(message)
}

fn gallery_error(error: DatabaseError) -> GalleryError {
    let message = error.to_string();
    drop(error);
    GalleryError::repository(message)
}

fn to_sql_error(error: DatabaseError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}
