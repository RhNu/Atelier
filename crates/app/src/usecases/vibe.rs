use super::{
    AppError, AppResult, Engine, EnsureVibeEncoding, EnsureVibeEncodingRequestDto,
    EnsuredVibeEncodingDto, ExportVibeDocument, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, GetVibeDocumentRequestDto, ImportEmbeddedPngVibeDocument,
    ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocument, ImportVibeDocumentRequestDto,
    ImportedVibeDocumentsDto, ListVibeDocumentsRequestDto, NovelAiClientFactory,
    RenameVibeDocumentRequestDto, STANDARD, SecretStore, SetVibeDocumentHiddenRequestDto,
    VibeDocumentEntryDto, VibeDocumentPageDto, VibeEncodeSettings, VibeId, VibeSourceIdentity,
    WorkspaceSession, ensured_vibe_to_dto, exported_vibe_to_dto, imported_vibes_to_dto,
    unix_timestamp_ms, vibe_entry_to_dto, vibe_format_to_domain, vibe_model_to_domain,
};
use atelier_vibe::VibeRepository;

pub struct VibeUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}

impl<S, F, E> VibeUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn import_document(
        &self,
        request: ImportVibeDocumentRequestDto,
    ) -> AppResult<ImportedVibeDocumentsDto> {
        let kernel = self.app.inner.kernel.lock().await;
        kernel
            .import_vibe_document(ImportVibeDocument {
                file_name: request.file_name,
                content: request.content,
            })
            .await
            .map(imported_vibes_to_dto)
            .map_err(AppError::from)
    }

    pub async fn import_embedded_png(
        &self,
        request: ImportEmbeddedPngVibeDocumentRequestDto,
    ) -> AppResult<ImportedVibeDocumentsDto> {
        let png_bytes = STANDARD.decode(request.png_bytes_base64)?;
        let kernel = self.app.inner.kernel.lock().await;
        kernel
            .import_embedded_png_vibe_document(ImportEmbeddedPngVibeDocument {
                file_name: request.file_name,
                png_bytes,
            })
            .await
            .map(imported_vibes_to_dto)
            .map_err(AppError::from)
    }

    pub async fn export_document(
        &self,
        request: ExportVibeDocumentRequestDto,
    ) -> AppResult<ExportedVibeDocumentDto> {
        let kernel = self.app.inner.kernel.lock().await;
        kernel
            .export_vibe_document(ExportVibeDocument {
                vibe_ids: request.vibe_ids.into_iter().map(VibeId::new).collect(),
                format: vibe_format_to_domain(request.format),
            })
            .await
            .map(exported_vibe_to_dto)
            .map_err(AppError::from)
    }

    pub async fn list_documents(
        &self,
        request: ListVibeDocumentsRequestDto,
    ) -> AppResult<VibeDocumentPageDto> {
        let (entries, total) = {
            let kernel = self.app.inner.kernel.lock().await;
            let entries = kernel
                .ports()
                .list_documents(request.offset, request.limit, request.include_hidden)
                .await
                .map_err(AppError::from)?;
            let total = kernel
                .ports()
                .count_documents(request.include_hidden)
                .await
                .map_err(AppError::from)?;
            drop(kernel);
            (entries, total)
        };
        Ok(VibeDocumentPageDto {
            items: entries.into_iter().map(vibe_entry_to_dto).collect(),
            total,
            offset: request.offset,
            limit: request.limit,
        })
    }

    pub async fn rename_document(
        &self,
        request: RenameVibeDocumentRequestDto,
    ) -> AppResult<VibeDocumentEntryDto> {
        let display_name = request.display_name.trim();
        if display_name.is_empty() {
            return Err(AppError::new(
                "vibe_invalid_settings",
                "vibe display name cannot be empty",
            ));
        }
        let entry = {
            let kernel = self.app.inner.kernel.lock().await;
            let entry = kernel
                .ports()
                .rename_document(
                    &VibeId::new(request.vibe_id),
                    display_name.to_owned(),
                    unix_timestamp_ms(),
                )
                .await
                .map_err(AppError::from)?;
            drop(kernel);
            entry.ok_or_else(|| AppError::new("vibe_not_found", "vibe document does not exist"))?
        };
        Ok(vibe_entry_to_dto(entry))
    }

    pub async fn set_document_hidden(
        &self,
        request: SetVibeDocumentHiddenRequestDto,
    ) -> AppResult<VibeDocumentEntryDto> {
        let entry = {
            let kernel = self.app.inner.kernel.lock().await;
            let entry = kernel
                .ports()
                .set_document_hidden(
                    &VibeId::new(request.vibe_id),
                    request.hidden,
                    unix_timestamp_ms(),
                )
                .await
                .map_err(AppError::from)?;
            drop(kernel);
            entry.ok_or_else(|| AppError::new("vibe_not_found", "vibe document does not exist"))?
        };
        Ok(vibe_entry_to_dto(entry))
    }

    pub async fn get_document(
        &self,
        request: GetVibeDocumentRequestDto,
    ) -> AppResult<VibeDocumentEntryDto> {
        let entry = {
            let kernel = self.app.inner.kernel.lock().await;
            let entry = kernel
                .ports()
                .get_document(&VibeId::new(request.vibe_id))
                .await
                .map_err(AppError::from)?;
            drop(kernel);
            entry.ok_or_else(|| AppError::new("vibe_not_found", "vibe document does not exist"))?
        };
        Ok(vibe_entry_to_dto(entry))
    }

    pub async fn ensure_encoding(
        &self,
        request: EnsureVibeEncodingRequestDto,
    ) -> AppResult<EnsuredVibeEncodingDto> {
        let settings = VibeEncodeSettings::new(
            vibe_model_to_domain(request.model),
            request.information_extracted,
        )?;
        let kernel = self.app.inner.kernel.lock().await;
        kernel
            .ensure_vibe_encoding(EnsureVibeEncoding {
                vibe_id: VibeId::new(request.vibe_id),
                source: VibeSourceIdentity::new_sha256(request.source_sha256),
                image: request.image,
                settings,
            })
            .await
            .map(|ensured| ensured_vibe_to_dto(&ensured))
            .map_err(AppError::from)
    }
}
