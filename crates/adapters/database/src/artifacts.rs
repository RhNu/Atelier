use async_trait::async_trait;
use atelier_artifacts::{
    ArtifactError, ArtifactId, ArtifactRecord, ArtifactRepository, ArtifactResult,
};
use atelier_gallery::GallerySourceKind;
use rusqlite::{OptionalExtension, params};

use crate::codec::{ArtifactRecordDto, JsonCodec, artifact_kind_as_str, source_kind_as_str};
use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;

#[derive(Clone, Debug)]
pub struct DatabaseArtifactRepository {
    connection: DatabaseConnection,
}

impl DatabaseArtifactRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl ArtifactRepository for DatabaseArtifactRepository {
    async fn insert_artifact(&self, record: ArtifactRecord) -> ArtifactResult<()> {
        let json = ArtifactRecordDto::encode_domain(&record).map_err(artifact_error)?;
        let source_kind = GallerySourceKind::from_artifact_source(&record.source);
        let connection = self.connection.lock().map_err(artifact_error)?;
        connection
            .execute(
                r"
                INSERT INTO artifacts(
                    artifact_id, artifact_kind, source_kind, primary_resource_id,
                    primary_variant_id, record_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(artifact_id) DO UPDATE
                SET artifact_kind = excluded.artifact_kind,
                    source_kind = excluded.source_kind,
                    primary_resource_id = excluded.primary_resource_id,
                    primary_variant_id = excluded.primary_variant_id,
                    record_json = excluded.record_json
                ",
                params![
                    record.id.as_str(),
                    artifact_kind_as_str(record.kind),
                    source_kind_as_str(source_kind),
                    record.primary_resource.id.as_str(),
                    record
                        .primary_resource
                        .variant_id
                        .as_ref()
                        .map(atelier_resource_catalog::VariantId::as_str),
                    json,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }

    async fn get_artifact(&self, id: &ArtifactId) -> ArtifactResult<Option<ArtifactRecord>> {
        let connection = self.connection.lock().map_err(artifact_error)?;
        let json = connection
            .query_row(
                "SELECT record_json FROM artifacts WHERE artifact_id = ?1",
                params![id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?;
        drop(connection);
        json.map(|json| ArtifactRecordDto::decode_domain(&json).map_err(artifact_error))
            .transpose()
    }

    async fn delete_artifacts(&self, ids: &[ArtifactId]) -> ArtifactResult<usize> {
        if ids.is_empty() {
            return Ok(0);
        }

        let deleted = {
            let mut connection = self.connection.lock().map_err(artifact_error)?;
            let transaction = connection.transaction().map_err(sql_error)?;
            let mut deleted = 0;
            for id in ids {
                deleted += transaction
                    .execute(
                        "DELETE FROM artifacts WHERE artifact_id = ?1",
                        params![id.as_str()],
                    )
                    .map_err(sql_error)?;
            }
            transaction.commit().map_err(sql_error)?;
            drop(connection);
            deleted
        };
        Ok(deleted)
    }
}

fn sql_error(error: rusqlite::Error) -> ArtifactError {
    let message = error.to_string();
    drop(error);
    ArtifactError::repository(message)
}

fn artifact_error(error: DatabaseError) -> ArtifactError {
    let message = error.to_string();
    drop(error);
    ArtifactError::repository(message)
}
