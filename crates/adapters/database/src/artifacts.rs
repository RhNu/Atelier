use async_trait::async_trait;
use nai_atelier_artifacts::{ArtifactError, ArtifactRecord, ArtifactRepository, ArtifactResult};
use nai_atelier_gallery::GallerySourceKind;
use rusqlite::params;

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
                        .map(nai_atelier_resource_catalog::VariantId::as_str),
                    json,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
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
