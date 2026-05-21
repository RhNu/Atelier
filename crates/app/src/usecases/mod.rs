use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, SubscriptionSummaryDto, UpdateApiKeyRequestDto,
};
use nai_atelier_app_api::director::{
    DirectorToolDto, DirectorToolResultDto, RunDirectorToolRequestDto,
};
use nai_atelier_app_api::event::AppEventDto;
use nai_atelier_app_api::gallery::{GalleryPageDto, GalleryQueryDto, GallerySafetyOverrideDto};
use nai_atelier_app_api::generation::{
    CharacterDto, CharacterReferenceDto, CharacterReferenceTypeDto, ControlNetConfigDto,
    ControlNetInputDto, GenerateImageRequestDto, GenerateImageStreamRequestDto,
    GenerationStatusDto, GenerationWorkRequestDto, Img2ImgRequestDto, QueueDirectiveDto,
    SubmitGenerationRequestDto,
};
use nai_atelier_app_api::prompt::{
    CompilePromptRequestDto, CompiledPromptDto, DeletePromptChunkRequestDto,
    DeletePromptChunkResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
    PromptChunkDto, PromptChunkPageDto, PromptLexiconCatalogDto, PromptLexiconListQueryDto,
    PromptLexiconPageDto, UpsertPromptChunkRequestDto,
};
use nai_atelier_app_api::resource::ImageInputDto;
use nai_atelier_app_api::settings::{
    ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use nai_atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
    ImportedVibeDocumentsDto,
};
use nai_atelier_app_api::workspace::WorkspaceStatusDto;
use nai_atelier_artifacts::{ArtifactSource, VisualAssetRole};
use nai_atelier_director::{DirectorTool, RunDirectorToolRequest};
use nai_atelier_gallery::{GalleryItemId, GalleryQuery, GallerySourceKind};
use nai_atelier_generation::{
    Character, CharacterPosition, CharacterReference, CharacterReferenceType, ControlNetConfig,
    ControlNetInput, GenerateImageRequest, GenerateImageStreamRequest, ImageSize, Img2ImgRequest,
};
use nai_atelier_jobs::{
    BatchId, JobId, JobQueueRepository, JobStatus, RunHistoryKind, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus, RunOutputRecord,
};
use nai_atelier_kernel::{
    EnsureVibeEncoding, ExportVibeDocument, GenerationWorkRequest, ImportEmbeddedPngVibeDocument,
    ImportVibeDocument, RunDirectorTool, SubmitGenerationWork,
};
use nai_atelier_prompt_resources::{CompilePromptRequest, PromptChunkId, PromptChunkKey};
use nai_atelier_resource_catalog::ResourceVariantKind;
use nai_atelier_secrets::{ApiKeyId, SecretStore, SecretValue, SecretsErrorKind};
use nai_atelier_vibe::{VibeEncodeSettings, VibeId, VibeSourceIdentity};
use std::time::{SystemTime, UNIX_EPOCH};

mod history;
mod resource;

pub use history::{
    HistoryUseCases, ensure_generation_history_target_is_new,
    sync_generation_history_from_queue_snapshot, upsert_generation_history_record,
};
pub use resource::ResourceUseCases;

use crate::app::AtelierApp;
use crate::mapping::{
    api_key_record_to_dto, compiled_prompt_to_dto, create_api_key_to_domain, ensured_vibe_to_dto,
    exported_vibe_to_dto, gallery_image_reference_to_dto, gallery_item_to_dto, gallery_page_to_dto,
    gallery_query_to_domain, generation_status_to_dto, image_format_to_domain,
    image_model_to_domain, image_reference_target_to_domain, imported_vibes_to_dto,
    lexicon_catalog_to_dto, lexicon_page_to_dto, lexicon_query_to_domain, lexicon_search_to_page,
    noise_schedule_to_domain, plan_context_to_domain, prompt_chunk_to_dto, queue_directive_to_dto,
    resource_ref_from_dto, resource_ref_to_dto, safety_override_to_domain, sampler_to_domain,
    stream_mode_to_domain, subscription_to_dto, uc_preset_to_domain, upsert_prompt_chunk_to_domain,
    vibe_format_to_domain, vibe_model_to_domain, workspace_settings_to_domain,
    workspace_settings_to_dto,
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

pub struct SettingsUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> SettingsUseCases<'_, S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    pub async fn get(&self) -> AppResult<WorkspaceSettingsDto> {
        self.app
            .inner
            .settings
            .get_workspace_settings()
            .await
            .map(|settings| workspace_settings_to_dto(&settings))
            .map_err(AppError::from)
    }

    pub async fn update(
        &self,
        request: UpdateWorkspaceSettingsRequestDto,
    ) -> AppResult<WorkspaceSettingsDto> {
        let settings = workspace_settings_to_domain(&request.settings)?;
        self.app
            .inner
            .settings
            .update_workspace_settings(settings)
            .await
            .map(|settings| {
                self.app.inner.settings_state.replace(settings.clone());
                workspace_settings_to_dto(&settings)
            })
            .map_err(AppError::from)
    }

    pub async fn reset(&self) -> AppResult<ResetWorkspaceSettingsResponseDto> {
        self.app
            .inner
            .settings
            .reset_workspace_settings()
            .await
            .map(|settings| ResetWorkspaceSettingsResponseDto {
                settings: {
                    self.app.inner.settings_state.replace(settings.clone());
                    workspace_settings_to_dto(&settings)
                },
            })
            .map_err(AppError::from)
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
        let title = generation_work_title(&request.work);
        let batch_id = request.batch_id.clone();
        let job_id = request.job_id.clone();
        ensure_generation_history_target_is_new(&self.app.inner.run_history, &batch_id, &job_id)
            .await?;
        let work = self.submit_request_to_domain(request).await?;
        let mut kernel = self.app.inner.kernel.lock().await;
        let directive = kernel
            .submit_generation_work(work)
            .await
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        self.upsert_generation_history(
            &batch_id,
            &job_id,
            RunHistoryStatus::Queued,
            title,
            None,
            None,
        )
        .await?;
        Ok(directive)
    }

    pub async fn run_job(&self, job_id: &str) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let result = kernel
            .run_scheduled_generation_job(&JobId::new(job_id))
            .await;
        let snapshot = kernel.queue_snapshot();
        let job_status = kernel.job_status(&JobId::new(job_id));
        drop(kernel);

        let directive = match result {
            Ok(directive) => queue_directive_to_dto(directive),
            Err(error) => {
                let status =
                    job_status.map_or(RunHistoryStatus::Failed, run_history_status_from_job_status);
                self.update_generation_history_status(job_id, status, Some(error.to_string()))
                    .await?;
                self.persist_queue_snapshot(&QueueDirectiveDto::Idle, &snapshot)
                    .await?;
                return Err(AppError::from(error));
            }
        };
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        let status = job_status.map_or(
            RunHistoryStatus::Succeeded,
            run_history_status_from_job_status,
        );
        self.update_generation_history_status(job_id, status, None)
            .await?;
        self.persist_generation_outputs(job_id).await?;
        Ok(directive)
    }

    pub async fn pause(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let directive = kernel
            .pause()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        Ok(directive)
    }

    pub async fn resume(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let directive = kernel
            .resume()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        Ok(directive)
    }

    pub async fn stop(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let directive = kernel
            .stop()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        Ok(directive)
    }

    pub async fn delay_elapsed(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let directive = kernel
            .delay_elapsed()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        Ok(directive)
    }

    pub async fn status(&self, job_id: Option<&str>) -> GenerationStatusDto {
        let kernel = self.app.inner.kernel.lock().await;
        generation_status_to_dto(
            kernel.batch_status(),
            job_id.and_then(|id| kernel.job_status(&JobId::new(id))),
        )
    }

    async fn submit_request_to_domain(
        &self,
        request: SubmitGenerationRequestDto,
    ) -> AppResult<SubmitGenerationWork> {
        Ok(SubmitGenerationWork {
            batch_id: BatchId::new(request.batch_id),
            job_id: JobId::new(request.job_id),
            request: self.work_request_to_domain(request.work).await?,
            context: plan_context_to_domain(request.context),
        })
    }

    async fn work_request_to_domain(
        &self,
        value: GenerationWorkRequestDto,
    ) -> AppResult<GenerationWorkRequest> {
        match value {
            GenerationWorkRequestDto::Image(request) => Ok(GenerationWorkRequest::Image(
                self.generate_request_to_domain(request).await?,
            )),
            GenerationWorkRequestDto::Stream(request) => Ok(GenerationWorkRequest::Stream(
                self.stream_request_to_domain(request).await?,
            )),
        }
    }

    async fn stream_request_to_domain(
        &self,
        value: GenerateImageStreamRequestDto,
    ) -> AppResult<GenerateImageStreamRequest> {
        Ok(GenerateImageStreamRequest {
            base: self.generate_request_to_domain(value.base).await?,
            stream: stream_mode_to_domain(value.stream),
        })
    }

    async fn generate_request_to_domain(
        &self,
        value: GenerateImageRequestDto,
    ) -> AppResult<GenerateImageRequest> {
        Ok(GenerateImageRequest {
            prompt: value.prompt,
            model: image_model_to_domain(value.model),
            size: ImageSize {
                width: value.size.width,
                height: value.size.height,
            },
            negative_prompt: value.negative_prompt,
            quality: value.quality,
            uc_preset: uc_preset_to_domain(value.uc_preset),
            steps: value.steps,
            scale: value.scale,
            sampler: sampler_to_domain(value.sampler),
            noise_schedule: noise_schedule_to_domain(value.noise_schedule),
            seed: value.seed,
            n_samples: value.n_samples,
            cfg_rescale: value.cfg_rescale,
            variety_boost: value.variety_boost,
            i2i: self.optional_i2i_to_domain(value.i2i).await?,
            controlnet: value.controlnet.map(controlnet_to_domain),
            character_references: self
                .optional_character_references_to_domain(value.character_references)
                .await?,
            characters: value.characters.map(characters_to_domain),
            use_coords: value.use_coords,
            image_format: value.image_format.map(image_format_to_domain),
            strict_mode: value.strict_mode,
        })
    }

    async fn optional_i2i_to_domain(
        &self,
        value: Option<Img2ImgRequestDto>,
    ) -> AppResult<Option<Img2ImgRequest>> {
        match value {
            Some(request) => self.i2i_to_domain(request).await.map(Some),
            None => Ok(None),
        }
    }

    async fn i2i_to_domain(&self, value: Img2ImgRequestDto) -> AppResult<Img2ImgRequest> {
        let mask = match value.mask {
            Some(mask) => Some(self.image_input_to_base64(mask).await?),
            None => None,
        };
        Ok(Img2ImgRequest {
            image: self.image_input_to_base64(value.image).await?,
            strength: value.strength,
            noise: value.noise,
            mask,
        })
    }

    async fn optional_character_references_to_domain(
        &self,
        value: Option<Vec<CharacterReferenceDto>>,
    ) -> AppResult<Option<Vec<CharacterReference>>> {
        let Some(references) = value else {
            return Ok(None);
        };
        let mut resolved = Vec::with_capacity(references.len());
        for reference in references {
            resolved.push(self.character_reference_to_domain(reference).await?);
        }
        Ok(Some(resolved))
    }

    async fn character_reference_to_domain(
        &self,
        value: CharacterReferenceDto,
    ) -> AppResult<CharacterReference> {
        Ok(CharacterReference {
            image: self.image_input_to_base64(value.image).await?,
            reference_type: character_reference_type_to_domain(value.reference_type),
            fidelity: value.fidelity,
            strength: value.strength,
        })
    }

    async fn image_input_to_base64(&self, input: ImageInputDto) -> AppResult<String> {
        match input {
            ImageInputDto::InlineBase64 { image_base64 } => Ok(image_base64),
            ImageInputDto::ResourceRef { resource } => {
                let reference = resource_ref_from_dto(resource);
                let kernel = self.app.inner.kernel.lock().await;
                kernel
                    .ports()
                    .resource_reader
                    .read_resource_base64(&reference)
                    .await
                    .map_err(AppError::from)
            }
        }
    }

    async fn persist_queue_snapshot(
        &self,
        directive: &QueueDirectiveDto,
        snapshot: &nai_atelier_jobs::JobQueueSnapshot,
    ) -> AppResult<()> {
        sync_generation_history_from_queue_snapshot(&self.app.inner.run_history, snapshot).await?;
        if matches!(directive, QueueDirectiveDto::Idle) {
            self.app
                .inner
                .queue_repository
                .clear_queue_snapshot()
                .await
                .map_err(|error| AppError::new("job_queue", error.to_string()))
        } else {
            self.app
                .inner
                .queue_repository
                .save_queue_snapshot(snapshot)
                .await
                .map_err(|error| AppError::new("job_queue", error.to_string()))
        }
    }

    async fn upsert_generation_history(
        &self,
        batch_id: &str,
        job_id: &str,
        status: RunHistoryStatus,
        title: Option<String>,
        origin_run_id: Option<String>,
        last_error: Option<String>,
    ) -> AppResult<RunHistoryRecord> {
        upsert_generation_history_record(
            &self.app.inner.run_history,
            batch_id,
            job_id,
            status,
            title,
            origin_run_id,
            last_error,
        )
        .await
    }

    async fn update_generation_history_status(
        &self,
        job_id: &str,
        status: RunHistoryStatus,
        last_error: Option<String>,
    ) -> AppResult<()> {
        let Some(existing) = self
            .app
            .inner
            .run_history
            .get_run_history(job_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?
        else {
            return Ok(());
        };
        self.upsert_generation_history(
            existing.batch_id.as_deref().unwrap_or(""),
            job_id,
            status,
            existing.title,
            existing.origin_run_id,
            last_error,
        )
        .await?;
        Ok(())
    }

    async fn persist_generation_outputs(&self, job_id: &str) -> AppResult<()> {
        let mut offset = 0;
        loop {
            let items = self
                .app
                .inner
                .gallery
                .query(GalleryQuery {
                    offset,
                    source_kind: Some(GallerySourceKind::Generation),
                    ..GalleryQuery::default()
                })
                .await?;
            if items.is_empty() {
                break;
            }
            let item_count = items.len();
            for item in items {
                let ArtifactSource::GenerationJob {
                    job_id: source_job_id,
                    ..
                } = &item.source
                else {
                    continue;
                };
                if source_job_id != job_id {
                    continue;
                }
                for asset in &item.assets {
                    self.app
                        .inner
                        .run_history
                        .upsert_run_output(RunOutputRecord {
                            run_id: job_id.to_owned(),
                            artifact_id: item.artifact_id.as_str().to_owned(),
                            item_id: Some(item.id.as_str().to_owned()),
                            resource_id: asset.resource.id.as_str().to_owned(),
                            variant_id: asset
                                .resource
                                .variant_id
                                .as_ref()
                                .map(|id| id.as_str().to_owned()),
                            asset_role: visual_asset_role_as_str(asset.role).to_owned(),
                            variant_kind: asset
                                .variant_kind
                                .map(resource_variant_kind_as_str)
                                .map(str::to_owned),
                        })
                        .await
                        .map_err(|error| AppError::new("run_history", error.to_string()))?;
                }
            }
            if item_count < GalleryQuery::default().limit {
                break;
            }
            offset += item_count;
        }
        Ok(())
    }
}

pub struct DirectorUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> DirectorUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    pub async fn run_tool(
        &self,
        request: RunDirectorToolRequestDto,
    ) -> AppResult<DirectorToolResultDto> {
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
        let run_id = request.run_id.clone();
        let title = Some(format!("{:?}", request.tool).to_lowercase());
        let image = match self.image_input_to_base64(request.image).await {
            Ok(image) => image,
            Err(error) => {
                self.upsert_director_history(
                    &run_id,
                    title.clone(),
                    RunHistoryStatus::Failed,
                    Some(error.to_string()),
                )
                .await?;
                return Err(error);
            }
        };
        let work = RunDirectorTool {
            run_id: request.run_id,
            request: RunDirectorToolRequest {
                tool: director_tool_to_domain(request.tool),
                image,
                prompt: request.prompt,
                defry: request.defry,
                strict_mode: request.strict_mode,
            },
        };
        let mut kernel = self.app.inner.kernel.lock().await;
        let result = match kernel.run_director_tool(work).await {
            Ok(result) => result,
            Err(error) => {
                drop(kernel);
                let app_error = AppError::from(error);
                self.upsert_director_history(
                    &run_id,
                    title,
                    RunHistoryStatus::Failed,
                    Some(app_error.to_string()),
                )
                .await?;
                return Err(app_error);
            }
        };
        drop(kernel);
        self.upsert_director_history(&run_id, title, RunHistoryStatus::Succeeded, None)
            .await?;
        for asset in &result.item.assets {
            self.app
                .inner
                .run_history
                .upsert_run_output(RunOutputRecord {
                    run_id: run_id.clone(),
                    artifact_id: result.artifact_id.as_str().to_owned(),
                    item_id: Some(result.item.id.as_str().to_owned()),
                    resource_id: asset.resource.id.as_str().to_owned(),
                    variant_id: asset
                        .resource
                        .variant_id
                        .as_ref()
                        .map(|id| id.as_str().to_owned()),
                    asset_role: visual_asset_role_as_str(asset.role).to_owned(),
                    variant_kind: asset
                        .variant_kind
                        .map(resource_variant_kind_as_str)
                        .map(str::to_owned),
                })
                .await
                .map_err(|error| AppError::new("run_history", error.to_string()))?;
        }
        Ok(DirectorToolResultDto {
            item_id: result.item.id.as_str().to_owned(),
            artifact_id: result.artifact_id.as_str().to_owned(),
            resource: resource_ref_to_dto(&result.resource),
            item: gallery_item_to_dto(result.item),
        })
    }

    async fn upsert_director_history(
        &self,
        run_id: &str,
        title: Option<String>,
        status: RunHistoryStatus,
        last_error: Option<String>,
    ) -> AppResult<()> {
        let now = unix_timestamp_ms();
        let existing = self
            .app
            .inner
            .run_history
            .get_run_history(run_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        self.app
            .inner
            .run_history
            .upsert_run_history(RunHistoryRecord {
                run_id: run_id.to_owned(),
                kind: RunHistoryKind::Director,
                status,
                batch_id: None,
                job_id: None,
                origin_run_id: None,
                submitted_payload_ref: None,
                prepared_payload_ref: None,
                title: title.or_else(|| existing.as_ref().and_then(|record| record.title.clone())),
                last_error,
                created_at_ms: existing.as_ref().map_or(now, |record| record.created_at_ms),
                updated_at_ms: now,
                completed_at_ms: Some(now),
                recoverable: false,
            })
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))
    }

    async fn image_input_to_base64(&self, input: ImageInputDto) -> AppResult<String> {
        match input {
            ImageInputDto::InlineBase64 { image_base64 } => Ok(image_base64),
            ImageInputDto::ResourceRef { resource } => {
                let reference = resource_ref_from_dto(resource);
                let kernel = self.app.inner.kernel.lock().await;
                kernel
                    .ports()
                    .resource_reader
                    .read_resource_base64(&reference)
                    .await
                    .map_err(AppError::from)
            }
        }
    }
}

fn controlnet_to_domain(value: ControlNetConfigDto) -> ControlNetConfig {
    ControlNetConfig {
        images: value
            .images
            .into_iter()
            .map(controlnet_input_to_domain)
            .collect(),
        strength: value.strength,
    }
}

fn controlnet_input_to_domain(value: ControlNetInputDto) -> ControlNetInput {
    ControlNetInput {
        vibe_data_cache: value.vibe_data_cache,
        info_extracted: value.info_extracted,
        strength: value.strength,
    }
}

fn characters_to_domain(value: Vec<CharacterDto>) -> Vec<Character> {
    value
        .into_iter()
        .map(|character| Character {
            prompt: character.prompt,
            negative_prompt: character.negative_prompt,
            position: CharacterPosition {
                x: character.position.x,
                y: character.position.y,
            },
            enabled: character.enabled,
        })
        .collect()
}

const fn character_reference_type_to_domain(
    value: CharacterReferenceTypeDto,
) -> CharacterReferenceType {
    match value {
        CharacterReferenceTypeDto::Character => CharacterReferenceType::Character,
        CharacterReferenceTypeDto::Style => CharacterReferenceType::Style,
        CharacterReferenceTypeDto::CharacterAndStyle => CharacterReferenceType::CharacterAndStyle,
    }
}

const fn director_tool_to_domain(value: DirectorToolDto) -> DirectorTool {
    match value {
        DirectorToolDto::Lineart => DirectorTool::Lineart,
        DirectorToolDto::Sketch => DirectorTool::Sketch,
        DirectorToolDto::BgRemoval => DirectorTool::BgRemoval,
        DirectorToolDto::Emotion => DirectorTool::Emotion,
        DirectorToolDto::Declutter => DirectorTool::Declutter,
        DirectorToolDto::Colorize => DirectorTool::Colorize,
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

fn generation_work_title(work: &GenerationWorkRequestDto) -> Option<String> {
    let prompt = match work {
        GenerationWorkRequestDto::Image(request) => &request.prompt,
        GenerationWorkRequestDto::Stream(request) => &request.base.prompt,
    };
    (!prompt.trim().is_empty()).then(|| prompt.clone())
}

const fn run_history_status_from_job_status(status: JobStatus) -> RunHistoryStatus {
    match status {
        JobStatus::Queued => RunHistoryStatus::Queued,
        JobStatus::Preparing => RunHistoryStatus::Preparing,
        JobStatus::Running => RunHistoryStatus::Running,
        JobStatus::WaitingRetry => RunHistoryStatus::Waiting,
        JobStatus::Blocked => RunHistoryStatus::Paused,
        JobStatus::Succeeded => RunHistoryStatus::Succeeded,
        JobStatus::Failed => RunHistoryStatus::Failed,
        JobStatus::Skipped => RunHistoryStatus::Skipped,
    }
}

const fn visual_asset_role_as_str(value: VisualAssetRole) -> &'static str {
    match value {
        VisualAssetRole::Original => "original",
        VisualAssetRole::Thumbnail => "thumbnail",
        VisualAssetRole::Preview => "preview",
        VisualAssetRole::Sanitized => "sanitized",
        VisualAssetRole::Export => "export",
    }
}

const fn resource_variant_kind_as_str(value: ResourceVariantKind) -> &'static str {
    match value {
        ResourceVariantKind::Original => "original",
        ResourceVariantKind::Preview => "preview",
        ResourceVariantKind::Thumbnail => "thumbnail",
        ResourceVariantKind::Sanitized => "sanitized",
        ResourceVariantKind::Export => "export",
    }
}

fn unix_timestamp_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
