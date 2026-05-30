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

    fn update_document(
        &self,
        id: &VibeId,
        updated_at_ms: u64,
        update: impl FnOnce(&mut VibeDocumentEntry),
    ) -> VibeDomainResult<Option<VibeDocumentEntry>> {
        let connection = self.connection.lock().map_err(vibe_error)?;
        let Some(json) = connection
            .query_row(
                "SELECT document_json FROM vibe_documents WHERE vibe_id = ?1",
                params![id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error)?
        else {
            return Ok(None);
        };
        let mut entry = VibeDocumentEntryDto::decode_domain(&json).map_err(vibe_error)?;
        update(&mut entry);
        if entry.summary.display_name.trim().is_empty() {
            return Err(VibeError::invalid_settings(
                "vibe display name cannot be empty",
            ));
        }
        entry.summary.updated_at_ms = updated_at_ms;
        let json = VibeDocumentEntryDto::encode_domain(&entry).map_err(vibe_error)?;
        connection
            .execute(
                r"
                UPDATE vibe_documents
                SET display_name = ?2,
                    has_image = ?3,
                    document_json = ?4
                WHERE vibe_id = ?1
                ",
                params![
                    id.as_str(),
                    entry.summary.display_name.as_str(),
                    i64::from(entry.summary.has_image),
                    json,
                ],
            )
            .map_err(sql_error)?;
        Ok(Some(entry))
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
        include_hidden: bool,
    ) -> VibeDomainResult<Vec<VibeDocumentEntry>> {
        let connection = self.connection.lock().map_err(vibe_error)?;
        let mut statement = connection
            .prepare(
                r"
                SELECT document_json
                FROM vibe_documents
                ORDER BY display_name ASC, vibe_id ASC
                ",
            )
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(sql_error)?;
        let entries = rows
            .map(|row| {
                row.map_err(sql_error)
                    .and_then(|text| VibeDocumentEntryDto::decode_domain(&text).map_err(vibe_error))
            })
            .collect::<VibeDomainResult<Vec<_>>>()?;
        Ok(entries
            .into_iter()
            .filter(|entry| include_hidden || !entry.summary.hidden)
            .skip(offset)
            .take(limit)
            .collect())
    }

    async fn count_documents(&self, include_hidden: bool) -> VibeDomainResult<usize> {
        let connection = self.connection.lock().map_err(vibe_error)?;
        let mut statement = connection
            .prepare("SELECT document_json FROM vibe_documents")
            .map_err(sql_error)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(sql_error)?;
        let entries = rows
            .map(|row| {
                row.map_err(sql_error)
                    .and_then(|text| VibeDocumentEntryDto::decode_domain(&text).map_err(vibe_error))
            })
            .collect::<VibeDomainResult<Vec<_>>>()?;
        Ok(entries
            .into_iter()
            .filter(|entry| include_hidden || !entry.summary.hidden)
            .count())
    }

    async fn rename_document(
        &self,
        id: &VibeId,
        display_name: String,
        updated_at_ms: u64,
    ) -> VibeDomainResult<Option<VibeDocumentEntry>> {
        self.update_document(id, updated_at_ms, |entry| {
            display_name
                .trim()
                .clone_into(&mut entry.summary.display_name);
        })
    }

    async fn set_document_hidden(
        &self,
        id: &VibeId,
        hidden: bool,
        updated_at_ms: u64,
    ) -> VibeDomainResult<Option<VibeDocumentEntry>> {
        self.update_document(id, updated_at_ms, |entry| {
            entry.summary.hidden = hidden;
        })
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
