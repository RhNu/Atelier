#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use atelier_vibe::{
    VibeDocumentEntry, VibeDomainResult, VibeEncodeSettings, VibeEncodingRecord, VibeError,
    VibeErrorKind, VibeId, VibeRepository, VibeSourceIdentity,
};
use rusqlite::{OptionalExtension, params};

use crate::codec::{JsonCodec, VibeDocumentEntryDto, VibeEncodingRecordDto, vibe_model_as_str};
use crate::connection::DatabaseConnection;
use crate::error::DatabaseError;

#[derive(Clone, Debug)]
pub struct DatabaseVibeRepository {
    connection: DatabaseConnection,
}

impl DatabaseVibeRepository {
    #[must_use]
    pub const fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl VibeRepository for DatabaseVibeRepository {
    async fn insert_document(&self, entry: VibeDocumentEntry) -> VibeDomainResult<VibeId> {
        let json = VibeDocumentEntryDto::encode_domain(&entry).map_err(vibe_error)?;
        let connection = self.connection.lock().map_err(vibe_error)?;
        connection
            .execute(
                r"
                INSERT INTO vibe_documents(vibe_id, display_name, has_image, document_json)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(vibe_id) DO UPDATE
                SET display_name = excluded.display_name,
                    has_image = excluded.has_image,
                    document_json = excluded.document_json
                ",
                params![
                    entry.summary.document_id.as_str(),
                    entry.summary.display_name.as_str(),
                    i64::from(entry.summary.has_image),
                    json,
                ],
            )
            .map_err(sql_error)?;
        Ok(entry.summary.document_id)
    }

    async fn get_document(&self, id: &VibeId) -> VibeDomainResult<Option<VibeDocumentEntry>> {
        let connection = self.connection.lock().map_err(vibe_error)?;
        let json = connection
            .query_row(
                "SELECT document_json FROM vibe_documents WHERE vibe_id = ?1",
                params![id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?;
        json.map(|text| VibeDocumentEntryDto::decode_domain(&text).map_err(vibe_error))
            .transpose()
    }

    async fn list_documents(
        &self,
        offset: usize,
        limit: usize,
    ) -> VibeDomainResult<Vec<VibeDocumentEntry>> {
        let connection = self.connection.lock().map_err(vibe_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT document_json
                FROM vibe_documents
                ORDER BY display_name ASC, vibe_id ASC
                LIMIT ?1 OFFSET ?2
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map(
                params![
                    i64::try_from(limit).unwrap_or(i64::MAX),
                    i64::try_from(offset).unwrap_or(i64::MAX),
                ],
                |row| row.get::<_, String>(0),
            )
            .map_err(sql_error)?;
        rows.map(|row| {
            row.map_err(sql_error)
                .and_then(|text| VibeDocumentEntryDto::decode_domain(&text).map_err(vibe_error))
        })
        .collect()
    }

    async fn count_documents(&self) -> VibeDomainResult<usize> {
        let connection = self.connection.lock().map_err(vibe_error)?;
        let count = connection
            .query_row("SELECT COUNT(*) FROM vibe_documents", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(sql_error)?;
        usize::try_from(count)
            .map_err(|error| VibeError::new(VibeErrorKind::Repository, error.to_string()))
    }

    async fn find_cached_encoding(
        &self,
        source: &VibeSourceIdentity,
        settings: &VibeEncodeSettings,
    ) -> VibeDomainResult<Option<VibeEncodingRecord>> {
        let cache_key = settings.cache_key(source);
        let connection = self.connection.lock().map_err(vibe_error)?;
        let json = connection
            .query_row(
                "SELECT record_json FROM vibe_encodings WHERE cache_key = ?1",
                params![cache_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?;
        json.map(|text| VibeEncodingRecordDto::decode_domain(&text).map_err(vibe_error))
            .transpose()
    }

    async fn save_encoding(&self, record: VibeEncodingRecord) -> VibeDomainResult<()> {
        let cache_key = record.settings.cache_key(&record.source);
        let json = VibeEncodingRecordDto::encode_domain(&record).map_err(vibe_error)?;
        let connection = self.connection.lock().map_err(vibe_error)?;
        connection
            .execute(
                r"
                INSERT INTO vibe_encodings(
                    cache_key, vibe_id, source_hash, model, information_extracted_key, record_json
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(cache_key) DO UPDATE
                SET vibe_id = excluded.vibe_id,
                    source_hash = excluded.source_hash,
                    model = excluded.model,
                    information_extracted_key = excluded.information_extracted_key,
                    record_json = excluded.record_json
                ",
                params![
                    cache_key,
                    record.vibe_id.as_str(),
                    record.source.content_hash.as_str(),
                    vibe_model_as_str(record.settings.model),
                    record.settings.information_extracted_key(),
                    json,
                ],
            )
            .map(|_| ())
            .map_err(sql_error)
    }
}

fn sql_error(error: rusqlite::Error) -> VibeError {
    let message = error.to_string();
    drop(error);
    VibeError::new(VibeErrorKind::Repository, message)
}

fn vibe_error(error: DatabaseError) -> VibeError {
    let message = error.to_string();
    drop(error);
    VibeError::new(VibeErrorKind::Repository, message)
}
