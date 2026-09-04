use super::generation_support::{
    ensure_generation_batch_target_is_new, estimate_generation_anlas, parse_uc_preset_override,
};
use super::{
    AppError, AppResult, ArtifactSource, BatchId, CharacterReference, CharacterReferenceDto,
    CompileCharacterPromptRequest, CompileGenerationPromptRequest, GalleryQuery, GallerySourceKind,
    GenerateImageRequest, GenerateImageRequestDto, GenerateImageStreamRequest,
    GenerateImageStreamRequestDto, GenerationAnlasEstimateDto, GenerationEstimateRequestDto,
    GenerationHistoryPosition, GenerationHistoryUpdate, GenerationStatusDto, GenerationWorkRequest,
    GenerationWorkRequestDto, ImageInputDto, ImageSize, Img2ImgRequest, Img2ImgRequestDto, JobId,
    NovelAiClientFactory, PromptPresetId, QueueDirectiveDto, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus, RunOutputRecord, RunOutputState, SecretStore,
    SecretsErrorKind, SubmitGenerationBatch, SubmitGenerationBatchJob, SubmitGenerationBatchJobDto,
    SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto, VibeReference, VibeTransferConfig,
    VibeTransferConfigDto, WorkspaceSession, character_reference_type_to_domain,
    characters_to_domain, generation_status_to_dto, generation_work_title, image_format_to_domain,
    image_model_to_domain, noise_schedule_to_domain, plan_context_to_domain,
    quality_preset_to_domain, quality_preset_to_dto, queue_directive_to_dto, resource_ref_from_dto,
    resource_variant_kind_as_str, run_history_status_from_job_status, sampler_to_domain,
    stream_mode_to_domain, uc_preset_to_domain, upsert_generation_history_record,
    visual_asset_role_as_str,
};
pub struct GenerationUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}
impl<S, F, E> GenerationUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn submit(
        &self,
        request: SubmitGenerationRequestDto,
    ) -> AppResult<QueueDirectiveDto> {
        self.submit_batch(SubmitGenerationBatchRequestDto {
            batch_id: request.batch_id,
            jobs: vec![SubmitGenerationBatchJobDto {
                job_id: request.job_id,
                work: request.work,
            }],
            context: request.context,
        })
        .await
    }

    pub async fn submit_batch(
        &self,
        request: SubmitGenerationBatchRequestDto,
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
        let batch_id = request.batch_id.clone();
        ensure_generation_batch_target_is_new(
            &self.app.inner.run_history,
            &batch_id,
            request.jobs.iter().map(|job| job.job_id.as_str()),
        )
        .await?;
        let history_positions = request
            .jobs
            .iter()
            .enumerate()
            .map(|(index, job)| {
                (
                    job.job_id.clone(),
                    generation_work_title(&job.work),
                    GenerationHistoryPosition {
                        request_index: u32::try_from(index).unwrap_or(u32::MAX),
                        expected_samples: generation_work_sample_count(&job.work),
                    },
                )
            })
            .collect::<Vec<_>>();
        let work = self.submit_batch_request_to_domain(request).await?;
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .submit_generation_batch(work)
            .await
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_or_restore(&directive, &snapshot, previous_snapshot)
            .await?;
        for (job_id, title, position) in history_positions {
            self.upsert_generation_history(
                &batch_id,
                &job_id,
                GenerationHistoryUpdate {
                    status: RunHistoryStatus::Queued,
                    title,
                    origin_run_id: None,
                    last_error: None,
                    position: Some(position),
                },
            )
            .await?;
        }
        Ok(directive)
    }

    pub async fn run_job(&self, job_id: &str) -> AppResult<QueueDirectiveDto> {
        struct NeverCancel;
        impl atelier_kernel::GenerationTaskCancellation for NeverCancel {
            fn is_cancelled(&self) -> bool {
                false
            }
        }
        self.run_job_cancellable(job_id, &NeverCancel).await
    }

    pub async fn run_job_cancellable(
        &self,
        job_id: &str,
        cancellation: &dyn atelier_kernel::GenerationTaskCancellation,
    ) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let result = kernel
            .run_scheduled_generation_job_cancellable(&JobId::new(job_id), cancellation)
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
                self.persist_queue_snapshot_after_failure(&snapshot).await?;
                return Err(AppError::from(error));
            }
        };
        self.persist_or_restore(&directive, &snapshot, previous_snapshot)
            .await?;
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
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .pause()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_or_restore(&directive, &snapshot, previous_snapshot)
            .await?;
        Ok(directive)
    }

    pub async fn resume(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .resume()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_or_restore(&directive, &snapshot, previous_snapshot)
            .await?;
        Ok(directive)
    }

    pub async fn stop(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .stop()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_or_restore(&directive, &snapshot, previous_snapshot)
            .await?;
        Ok(directive)
    }

    pub async fn delay_elapsed(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .delay_elapsed()
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_or_restore(&directive, &snapshot, previous_snapshot)
            .await?;
        Ok(directive)
    }

    pub async fn status(&self, job_id: Option<&str>) -> AppResult<GenerationStatusDto> {
        let snapshot = self
            .app
            .inner
            .kernel
            .lock()
            .await
            .queue_snapshot()
            .active_batch;
        let history = if let Some(active) = &snapshot {
            self.app
                .inner
                .run_history
                .list_run_history_by_batch(active.batch.batch_id.as_str())
                .await
                .map_err(|error| AppError::new("run_history", error.to_string()))?
        } else {
            Vec::new()
        };
        Ok(generation_status_to_dto(snapshot, &history, job_id))
    }

    /// Estimates `NovelAI` Anlas cost after applying prompt preset bindings.
    ///
    /// # Errors
    /// Returns an error when preset compilation or generation planning fails.
    pub async fn estimate(
        &self,
        request: GenerationEstimateRequestDto,
    ) -> AppResult<GenerationAnlasEstimateDto> {
        let request = GenerationEstimateRequestDto {
            request: self.apply_prompt_presets(request.request).await?,
            context: request.context,
        };
        estimate_generation_anlas(&request)
    }

    async fn submit_batch_request_to_domain(
        &self,
        request: SubmitGenerationBatchRequestDto,
    ) -> AppResult<SubmitGenerationBatch> {
        let mut jobs = Vec::with_capacity(request.jobs.len());
        for job in request.jobs {
            jobs.push(SubmitGenerationBatchJob {
                job_id: JobId::new(job.job_id),
                request: self.work_request_to_domain(job.work).await?,
            });
        }
        Ok(SubmitGenerationBatch {
            batch_id: BatchId::new(request.batch_id),
            jobs,
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
        let value = self.apply_prompt_presets(value).await?;
        Ok(GenerateImageRequest {
            prompt: value.prompt,
            furry_mode: value.furry_mode,
            model: image_model_to_domain(value.model),
            size: ImageSize {
                width: value.size.width,
                height: value.size.height,
            },
            negative_prompt: value.negative_prompt,
            quality: quality_preset_to_domain(value.quality),
            transparent_background: value.transparent_background,
            uc_preset: uc_preset_to_domain(value.uc_preset),
            steps: value.steps,
            scale: value.scale,
            sampler: sampler_to_domain(value.sampler),
            noise_schedule: noise_schedule_to_domain(value.noise_schedule),
            seed: value.seed,
            n_samples: value.n_samples,
            cfg_rescale: value.cfg_rescale,
            variety_boost: value.variety_boost,
            img2img: self.optional_i2i_to_domain(value.img2img).await?,
            vibe_transfer: self
                .optional_vibe_transfer_to_domain(value.vibe_transfer)
                .await?,
            character_references: self
                .optional_character_references_to_domain(value.character_references)
                .await?,
            characters: value.characters.map(characters_to_domain),
            use_coords: value.use_coords,
            image_format: value.image_format.map(image_format_to_domain),
            strict_mode: value.strict_mode,
        })
    }

    async fn apply_prompt_presets(
        &self,
        mut value: GenerateImageRequestDto,
    ) -> AppResult<GenerateImageRequestDto> {
        let has_character_presets = value.characters.as_ref().is_some_and(|characters| {
            characters
                .iter()
                .any(|character| character.preset_id.is_some())
        });
        if value.main_preset_id.is_none() && !has_character_presets {
            return Ok(value);
        }

        let character_inputs = value.characters.clone().unwrap_or_default();
        let compiled = self
            .app
            .inner
            .prompt_compiler
            .compile_generation_prompt(CompileGenerationPromptRequest {
                model: image_model_to_domain(value.model),
                main_preset_id: value.main_preset_id.take().map(PromptPresetId::new),
                prompt: value.prompt.clone(),
                negative_prompt: value.negative_prompt.clone().unwrap_or_default(),
                characters: character_inputs
                    .into_iter()
                    .enumerate()
                    .map(|(index, character)| CompileCharacterPromptRequest {
                        character_index: u32::try_from(index).unwrap_or(u32::MAX),
                        preset_id: character.preset_id.map(PromptPresetId::new),
                        prompt: character.prompt,
                        negative_prompt: character.negative_prompt.unwrap_or_default(),
                    })
                    .collect(),
                max_depth: 16,
            })
            .await?;

        value.prompt = compiled.prompt;
        value.negative_prompt =
            (!compiled.negative_prompt.trim().is_empty()).then_some(compiled.negative_prompt);
        if let Some(quality_override) = compiled.quality_override {
            value.quality = quality_preset_to_dto(quality_override);
        }
        if let Some(uc_preset_override) = compiled.uc_preset_override.as_deref() {
            value.uc_preset = parse_uc_preset_override(uc_preset_override)?;
        }
        if let Some(characters) = value.characters.as_mut() {
            for (index, character) in characters.iter_mut().enumerate() {
                if let Some(compiled_character) = compiled
                    .characters
                    .iter()
                    .find(|item| item.character_index == u32::try_from(index).unwrap_or(u32::MAX))
                {
                    character.prompt.clone_from(&compiled_character.prompt);
                    character.negative_prompt =
                        (!compiled_character.negative_prompt.trim().is_empty())
                            .then_some(compiled_character.negative_prompt.clone());
                }
                character.preset_id = None;
            }
        }
        Ok(value)
    }

    async fn optional_vibe_transfer_to_domain(
        &self,
        value: Option<VibeTransferConfigDto>,
    ) -> AppResult<Option<VibeTransferConfig>> {
        let Some(config) = value else {
            return Ok(None);
        };
        let mut references = Vec::with_capacity(config.references.len());
        for item in config.references {
            let reference = resource_ref_from_dto(item.encoding);
            let vibe_data_cache = self
                .app
                .inner
                .resource_reader
                .read_resource_base64(&reference)
                .await
                .map_err(AppError::from)?;
            references.push(VibeReference {
                vibe_data_cache,
                strength: item.strength,
            });
        }
        Ok(Some(VibeTransferConfig {
            references,
            strength: config.strength,
        }))
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
        let inpaint = match value.inpaint {
            Some(inpaint) => Some(atelier_generation::InpaintRequest {
                region_to_replace: self
                    .image_input_to_base64(inpaint.region_to_replace)
                    .await?,
            }),
            None => None,
        };
        Ok(Img2ImgRequest {
            image: self.image_input_to_base64(value.image).await?,
            strength: value.strength,
            noise: value.noise,
            inpaint,
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
                self.app
                    .inner
                    .resource_reader
                    .read_resource_base64(&reference)
                    .await
                    .map_err(AppError::from)
            }
        }
    }

    async fn upsert_generation_history(
        &self,
        batch_id: &str,
        job_id: &str,
        update: GenerationHistoryUpdate,
    ) -> AppResult<RunHistoryRecord> {
        upsert_generation_history_record(&self.app.inner.run_history, batch_id, job_id, update)
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
            GenerationHistoryUpdate {
                status,
                title: existing.title,
                origin_run_id: existing.origin_run_id,
                last_error,
                position: None,
            },
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
                            sample_index: item.metadata.sample_index,
                            artifact_id: item.artifact_id.as_str().to_owned(),
                            item_id: Some(item.id.as_str().to_owned()),
                            resource_id: Some(asset.resource.id.as_str().to_owned()),
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
                            state: RunOutputState::Available,
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

fn generation_work_sample_count(work: &GenerationWorkRequestDto) -> u32 {
    let count = match work {
        GenerationWorkRequestDto::Image(request) => request.n_samples,
        GenerationWorkRequestDto::Stream(request) => request.base.n_samples,
    };
    count.max(1)
}
