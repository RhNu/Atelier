use super::{
    AppError, AppResult, AtelierApp, Engine, EnsureVibeEncoding, EnsureVibeEncodingRequestDto,
    EnsuredVibeEncodingDto, ExportVibeDocument, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, ImportEmbeddedPngVibeDocument,
    ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocument, ImportVibeDocumentRequestDto,
    ImportedVibeDocumentsDto, NovelAiClientFactory, STANDARD, SecretStore, VibeEncodeSettings,
    VibeId, VibeSourceIdentity, ensured_vibe_to_dto, exported_vibe_to_dto, imported_vibes_to_dto,
    vibe_format_to_domain, vibe_model_to_domain,
};

pub struct VibeUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> VibeUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: nai_atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
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
