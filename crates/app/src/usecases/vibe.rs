use crate::mapping::{
    ensured_vibe_to_dto, exported_vibe_to_dto, imported_vibes_to_dto, vibe_entry_to_dto,
    vibe_format_to_domain, vibe_model_to_domain,
};
use crate::session::WorkspaceSession;
use crate::time::unix_timestamp_ms;
use crate::{AppError, AppResult};
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, GetVibeDocumentRequestDto, ImportEmbeddedPngVibeDocumentRequestDto,
    ImportVibeDocumentRequestDto, ImportedVibeDocumentsDto, ListVibeDocumentsRequestDto,
    RenameVibeDocumentRequestDto, SetVibeDocumentHiddenRequestDto, VibeDocumentEntryDto,
    VibeDocumentPageDto,
};
use atelier_kernel::{
    EnsureVibeEncoding, ExportVibeDocument, ImportEmbeddedPngVibeDocument, ImportVibeDocument,
};
use atelier_secrets::SecretStore;
use atelier_vibe::{VibeEncodeSettings, VibeId, VibeRepository, VibeSourceIdentity};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

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
        let kernel = &self.app.workflows;
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
        let kernel = &self.app.workflows;
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
        let kernel = &self.app.workflows;
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
            let kernel = &self.app.workflows;
            if let Some(model) = request.model {
                let model = vibe_model_to_domain(model);
                let all = kernel
                    .ports()
                    .list_documents(0, usize::MAX, request.include_hidden)
                    .await
                    .map_err(AppError::from)?;
                let filtered = all
                    .into_iter()
                    .filter(|entry| {
                        entry
                            .summary
                            .available_encoding_configs
                            .iter()
                            .any(|config| config.model == model)
                    })
                    .collect::<Vec<_>>();
                let total = filtered.len();
                let start = request.offset.min(total);
                let end = start.saturating_add(request.limit).min(total);
                (filtered[start..end].to_vec(), total)
            } else {
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
                (entries, total)
            }
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
            let kernel = &self.app.workflows;
            let entry = kernel
                .ports()
                .rename_document(
                    &VibeId::new(request.vibe_id),
                    display_name.to_owned(),
                    unix_timestamp_ms(),
                )
                .await
                .map_err(AppError::from)?;
            entry.ok_or_else(|| AppError::new("vibe_not_found", "vibe document does not exist"))?
        };
        Ok(vibe_entry_to_dto(entry))
    }

    pub async fn set_document_hidden(
        &self,
        request: SetVibeDocumentHiddenRequestDto,
    ) -> AppResult<VibeDocumentEntryDto> {
        let entry = {
            let kernel = &self.app.workflows;
            let entry = kernel
                .ports()
                .set_document_hidden(
                    &VibeId::new(request.vibe_id),
                    request.hidden,
                    unix_timestamp_ms(),
                )
                .await
                .map_err(AppError::from)?;
            entry.ok_or_else(|| AppError::new("vibe_not_found", "vibe document does not exist"))?
        };
        Ok(vibe_entry_to_dto(entry))
    }

    pub async fn get_document(
        &self,
        request: GetVibeDocumentRequestDto,
    ) -> AppResult<VibeDocumentEntryDto> {
        let entry = {
            let kernel = &self.app.workflows;
            let entry = kernel
                .ports()
                .get_document(&VibeId::new(request.vibe_id))
                .await
                .map_err(AppError::from)?;
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
        let kernel = &self.app.workflows;
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
