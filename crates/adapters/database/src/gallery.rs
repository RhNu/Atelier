#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use atelier_gallery::{
    GalleryError, GalleryIndex, GalleryItem, GalleryItemId, GalleryQuery, GalleryResult,
    GallerySafetyOverride,
};
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

impl DatabaseGalleryIndex {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
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
                    manual_safety_override, indexed_at_ms, item_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(item_id) DO UPDATE
                SET artifact_id = excluded.artifact_id,
                    artifact_kind = excluded.artifact_kind,
                    source_kind = excluded.source_kind,
                    manual_safety_override = excluded.manual_safety_override,
                    indexed_at_ms = excluded.indexed_at_ms,
                    item_json = excluded.item_json
                ",
                params![
                    item.id.as_str(),
                    item.artifact_id.as_str(),
                    artifact_kind_as_str(item.artifact_kind),
                    source_kind_as_str(item.source_kind()),
                    item.manual_safety_override.map(safety_override_as_str),
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
        let mut statement = connection
            .prepare(
                r"
                SELECT item_json, manual_safety_override
                FROM gallery_items
                WHERE (?1 IS NULL OR artifact_kind = ?1)
                  AND (?2 IS NULL OR source_kind = ?2)
                  AND (?3 IS NULL OR manual_safety_override = ?3)
                ORDER BY indexed_at_ms DESC, item_id ASC
                LIMIT ?4 OFFSET ?5
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(
                params![
                    artifact_kind,
                    source_kind,
                    safety_override,
                    i64::try_from(query.limit)
                        .map_err(|error| GalleryError::repository(error.to_string()))?,
                    i64::try_from(query.offset)
                        .map_err(|error| GalleryError::repository(error.to_string()))?,
                ],
                gallery_item_from_row,
            )
            .map_err(sql_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
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
                    item_json = ?3
                WHERE item_id = ?1
                ",
                params![
                    id.as_str(),
                    manual_safety_override.map(safety_override_as_str),
                    json,
                ],
            )
            .map_err(sql_error)?;
        Ok(item)
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
