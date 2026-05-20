use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, SubscriptionSummaryDto, UpdateApiKeyRequestDto,
};
use nai_atelier_app_api::event::AppEventDto;
use nai_atelier_app_api::gallery::{GalleryPageDto, GalleryQueryDto, GallerySafetyOverrideDto};
use nai_atelier_app_api::generation::{
    GenerationStatusDto, QueueDirectiveDto, SubmitGenerationRequestDto,
};
use nai_atelier_app_api::prompt::{
    CompilePromptRequestDto, CompiledPromptDto, DeletePromptChunkRequestDto,
    DeletePromptChunkResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
    PromptChunkDto, PromptChunkPageDto, PromptLexiconCatalogDto, PromptLexiconListQueryDto,
    PromptLexiconPageDto, UpsertPromptChunkRequestDto,
};
use nai_atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
    ImportedVibeDocumentsDto,
};
use nai_atelier_app_api::workspace::WorkspaceStatusDto;
use nai_atelier_gallery::GalleryItemId;
use nai_atelier_jobs::JobId;
use nai_atelier_kernel::{
    EnsureVibeEncoding, ExportVibeDocument, ImportEmbeddedPngVibeDocument, ImportVibeDocument,
};
use nai_atelier_prompt_resources::{CompilePromptRequest, PromptChunkId, PromptChunkKey};
use nai_atelier_secrets::{ApiKeyId, SecretStore, SecretValue, SecretsErrorKind};
use nai_atelier_vibe::{VibeEncodeSettings, VibeId, VibeSourceIdentity};

use crate::app::AtelierApp;
use crate::mapping::{
    api_key_record_to_dto, compiled_prompt_to_dto, create_api_key_to_domain, ensured_vibe_to_dto,
    exported_vibe_to_dto, gallery_image_reference_to_dto, gallery_item_to_dto, gallery_page_to_dto,
    gallery_query_to_domain, generation_status_to_dto, image_reference_target_to_domain,
    imported_vibes_to_dto, lexicon_catalog_to_dto, lexicon_page_to_dto, lexicon_query_to_domain,
    lexicon_search_to_page, prompt_chunk_to_dto, queue_directive_to_dto, safety_override_to_domain,
    submit_generation_to_domain, subscription_to_dto, upsert_prompt_chunk_to_domain,
    vibe_format_to_domain, vibe_model_to_domain,
};
use crate::{AppError, AppResult};

pub struct WorkspaceUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> WorkspaceUseCases<'_, S, F, E> {
    #[must_use]
    pub fn status(&self) -> WorkspaceStatusDto {
        WorkspaceStatusDto {
            root: self.app.inner.root.as_path().to_path_buf(),
            schema_version: self.app.inner.schema_version,
            locked: true,
        }
    }
}

pub struct AccountUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> AccountUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    pub async fn create_api_key(
        &self,
        request: CreateApiKeyRequestDto,
    ) -> AppResult<ApiKeyRecordDto> {
        self.app
            .inner
            .api_keys
            .create_api_key(create_api_key_to_domain(request))
            .await
            .map(|record| api_key_record_to_dto(&record))
            .map_err(AppError::from)
    }

    pub async fn update_api_key(
        &self,
        request: UpdateApiKeyRequestDto,
    ) -> AppResult<ApiKeyRecordDto> {
        self.app
            .inner
            .api_keys
            .update_api_key(nai_atelier_secrets::UpdateApiKeyRequest {
                id: ApiKeyId::new(request.id),
                display_name: request.display_name,
                secret: request.secret.map(SecretValue::new),
            })
            .await
            .map(|record| api_key_record_to_dto(&record))
            .map_err(AppError::from)
    }

    pub async fn delete_api_key(&self, id: &str) -> AppResult<bool> {
        self.app
            .inner
            .api_keys
            .delete_api_key(&ApiKeyId::new(id))
            .await
            .map_err(AppError::from)
    }

    pub async fn list_api_keys(&self) -> AppResult<Vec<ApiKeyRecordDto>> {
        self.app
            .inner
            .api_keys
            .list_api_keys()
            .await
            .map(|items| items.iter().map(api_key_record_to_dto).collect())
            .map_err(AppError::from)
    }

    pub async fn set_active_api_key(&self, id: &str) -> AppResult<()> {
        self.app
            .inner
            .api_keys
            .set_active_api_key(&ApiKeyId::new(id))
            .await
            .map_err(AppError::from)
    }

    pub async fn probe_key(&self, id: &str) -> AppResult<SubscriptionSummaryDto> {
        self.app
            .inner
            .api_keys
            .probe_key(&ApiKeyId::new(id))
            .await
            .map(|summary| subscription_to_dto(&summary))
            .map_err(AppError::from)
    }

    pub async fn probe_active(&self) -> AppResult<SubscriptionSummaryDto> {
        let active = self
            .app
            .inner
            .api_keys
            .list_api_keys()
            .await?
            .into_iter()
            .find(|record| record.is_active)
            .ok_or_else(AppError::missing_active_key)?;
        self.probe_key(active.id.as_str()).await
    }
}

pub struct PromptUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> PromptUseCases<'_, S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    pub async fn upsert_chunk(
        &self,
        request: UpsertPromptChunkRequestDto,
    ) -> AppResult<PromptChunkDto> {
        self.app
            .inner
            .prompt_chunks
            .upsert_chunk(upsert_prompt_chunk_to_domain(request)?)
            .await
            .map(|chunk| prompt_chunk_to_dto(&chunk))
            .map_err(AppError::from)
    }

    pub async fn get_chunk(&self, request: GetPromptChunkRequestDto) -> AppResult<PromptChunkDto> {
        let chunk = match (request.chunk_id, request.key) {
            (Some(id), None) => {
                self.app
                    .inner
                    .prompt_chunks
                    .get_chunk_by_id(&PromptChunkId::new(id))
                    .await?
            }
            (None, Some(key)) => {
                self.app
                    .inner
                    .prompt_chunks
                    .get_chunk_by_key(&PromptChunkKey::parse(&key)?)
                    .await?
            }
            _ => {
                return Err(AppError::new(
                    "invalid_request",
                    "provide exactly one of chunk_id or key",
                ));
            }
        };
        chunk
            .as_ref()
            .map(prompt_chunk_to_dto)
            .ok_or_else(|| AppError::new("prompt_not_found", "prompt chunk does not exist"))
    }

    pub async fn list_chunks(
        &self,
        request: ListPromptChunksRequestDto,
    ) -> AppResult<PromptChunkPageDto> {
        let chunks = self.app.inner.prompt_chunks.list_chunks().await?;
        let total = chunks.len();
        let start = request.offset.min(total);
        let end = start.saturating_add(request.limit).min(total);
        Ok(PromptChunkPageDto {
            items: chunks[start..end].iter().map(prompt_chunk_to_dto).collect(),
            total,
            offset: request.offset,
            limit: request.limit,
        })
    }

    pub async fn delete_chunk(
        &self,
        request: DeletePromptChunkRequestDto,
    ) -> AppResult<DeletePromptChunkResponseDto> {
        self.app
            .inner
            .prompt_chunks
            .delete_chunk(&PromptChunkId::new(request.chunk_id))
            .await
            .map(|result| DeletePromptChunkResponseDto {
                deleted: result.deleted,
            })
            .map_err(AppError::from)
    }

    pub async fn compile_preview(
        &self,
        request: CompilePromptRequestDto,
    ) -> AppResult<CompiledPromptDto> {
        self.app
            .inner
            .prompt_compiler
            .compile(CompilePromptRequest {
                prompt: request.prompt,
                max_depth: request.max_depth,
            })
            .await
            .map(|compiled| compiled_prompt_to_dto(&compiled))
            .map_err(AppError::from)
    }

    pub fn lexicon_catalog(&self) -> PromptLexiconCatalogDto {
        lexicon_catalog_to_dto(self.app.inner.lexicon.catalog())
    }

    pub fn lexicon_list(
        &self,
        query: PromptLexiconListQueryDto,
    ) -> AppResult<PromptLexiconPageDto> {
        self.app
            .inner
            .lexicon
            .list(&lexicon_query_to_domain(query))
            .map(lexicon_page_to_dto)
            .map_err(AppError::from)
    }

    pub fn lexicon_search(&self, query: &str, limit: usize) -> AppResult<PromptLexiconPageDto> {
        if limit == 0 {
            return Err(AppError::new(
                "invalid_request",
                "limit must be greater than zero",
            ));
        }
        Ok(lexicon_search_to_page(
            self.app.inner.lexicon.search(query, limit),
            limit,
        ))
    }
}

pub struct GenerationUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: nai_atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn submit(
        &self,
        request: SubmitGenerationRequestDto,
    ) -> AppResult<QueueDirectiveDto> {
        self.app
            .inner
            .api_keys
            .resolve_active_secret()
            .await
            .map_err(|error| {
                if error.kind == SecretsErrorKind::MissingActiveKey {
                    AppError::missing_active_key()
                } else {
                    AppError::from(error)
                }
            })?;
        let work = submit_generation_to_domain(request);
        let mut kernel = self.app.inner.kernel.lock().await;
        kernel
            .submit_generation_work(work)
            .await
            .map(queue_directive_to_dto)
            .map_err(AppError::from)
    }

    pub async fn run_job(&self, job_id: &str) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        kernel
            .run_scheduled_generation_job(&JobId::new(job_id))
            .await
            .map(queue_directive_to_dto)
            .map_err(AppError::from)
    }

    pub async fn pause(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        kernel
            .pause()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)
    }

    pub async fn resume(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        kernel
            .resume()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)
    }

    pub async fn stop(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        kernel
            .stop()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)
    }

    pub async fn delay_elapsed(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        kernel
            .delay_elapsed()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)
    }

    pub async fn status(&self, job_id: Option<&str>) -> GenerationStatusDto {
        let kernel = self.app.inner.kernel.lock().await;
        generation_status_to_dto(
            kernel.batch_status(),
            job_id.and_then(|id| kernel.job_status(&JobId::new(id))),
        )
    }
}

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

pub struct GalleryUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> GalleryUseCases<'_, S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    pub async fn query(&self, query: GalleryQueryDto) -> AppResult<GalleryPageDto> {
        let offset = query.offset;
        let limit = query.limit;
        let page_query = gallery_query_to_domain(&query)?;
        let total_query = gallery_query_to_domain(&GalleryQueryDto {
            offset: 0,
            limit: usize::try_from(i64::MAX).unwrap_or(usize::MAX),
            ..query
        })?;
        let items = self
            .app
            .inner
            .gallery
            .query(page_query)
            .await
            .map_err(AppError::from)?;
        let total = self
            .app
            .inner
            .gallery
            .query(total_query)
            .await
            .map_err(AppError::from)?
            .len();
        Ok(gallery_page_to_dto(items, offset, limit, total))
    }

    pub async fn set_safety_override(
        &self,
        item_id: &str,
        override_value: Option<GallerySafetyOverrideDto>,
    ) -> AppResult<nai_atelier_app_api::gallery::GalleryItemDto> {
        self.app
            .inner
            .gallery
            .set_safety_override(
                &GalleryItemId::new(item_id),
                override_value.map(safety_override_to_domain),
            )
            .await
            .map(gallery_item_to_dto)
            .map_err(AppError::from)
    }

    pub async fn image_reference(
        &self,
        request: nai_atelier_app_api::gallery::GalleryImageReferenceRequestDto,
    ) -> AppResult<nai_atelier_app_api::gallery::GalleryImageReferenceDto> {
        self.app
            .inner
            .gallery
            .image_reference_for(
                &GalleryItemId::new(request.item_id),
                image_reference_target_to_domain(request.target),
            )
            .await
            .map(gallery_image_reference_to_dto)
            .map_err(AppError::from)
    }
}

pub struct EventsUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> EventsUseCases<'_, S, F, E> {
    #[must_use]
    pub fn events_since(&self, sequence: u64, limit: usize) -> Vec<AppEventDto> {
        self.app.inner.events.events_since(sequence, limit)
    }
}
