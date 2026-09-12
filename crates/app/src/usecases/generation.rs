use super::generation_support::{
    ensure_generation_batch_target_is_new, estimate_generation_anlas, parse_uc_preset_override,
};
use super::{
    AppError, AppResult, BatchId, CharacterReference, CharacterReferenceDto, GenerateImageRequest,
    GenerateImageRequestDto, GenerateImageStreamRequest, GenerationAnlasEstimateDto,
    GenerationEstimateRequestDto, GenerationHistoryPosition, GenerationHistoryUpdate,
    GenerationStatusDto, GenerationWorkRequest, GenerationWorkRequestDto, ImageSize,
    Img2ImgRequest, Img2ImgRequestDto, JobId, NovelAiClientFactory, QueueDirectiveDto,
    RunHistoryRecord, RunHistoryRepository, RunHistoryStatus, SecretStore, SecretsErrorKind,
    SubmitGenerationBatch, SubmitGenerationBatchJob, SubmitGenerationBatchJobDto,
    SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto, VibeReference, VibeTransferConfig,
    VibeTransferConfigDto, WorkspaceSession, characters_to_domain, generation_status_to_dto,
    generation_work_title, image_format_to_domain, image_model_to_domain, noise_schedule_to_domain,
    plan_context_to_domain, quality_preset_to_domain, quality_preset_to_dto,
    queue_directive_to_dto, resource_ref_from_dto, run_history_status_from_job_status,
    sampler_to_domain, stream_mode_to_domain, uc_preset_to_domain,
    upsert_generation_history_record,
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
            &self.app.run_history,
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
        let mut kernel = self.app.kernel.lock().await;
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
        let mut kernel = self.app.kernel.lock().await;
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
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        let status = job_status.map_or(
            RunHistoryStatus::Succeeded,
            run_history_status_from_job_status,
        );
        self.update_generation_history_status(job_id, status, None)
            .await?;
        Ok(directive)
    }

    pub async fn pause(&self) -> AppResult<QueueDirectiveDto> {
        let mut kernel = self.app.kernel.lock().await;
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
        let mut kernel = self.app.kernel.lock().await;
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
        let mut kernel = self.app.kernel.lock().await;
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
        let mut kernel = self.app.kernel.lock().await;
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
        let snapshot = self.app.kernel.lock().await.queue_snapshot().active_batch;
        let history = if let Some(active) = &snapshot {
            self.app
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
            request: self.prepare_prompt(request.request).await?.0,
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
            let (work, compiled_prompt) = self.work_request_to_domain(job.work).await?;
            jobs.push(SubmitGenerationBatchJob {
                compiled_prompt: Some(compiled_prompt),
                job_id: JobId::new(job.job_id),
                request: work,
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
        request: GenerationWorkRequestDto,
    ) -> AppResult<(
        GenerationWorkRequest,
        atelier_prompt_resources::CompiledPrompt,
    )> {
        match request {
            GenerationWorkRequestDto::Image(request) => {
                let (request, prompt) = self.prepare_prompt(request).await?;
                Ok((
                    GenerationWorkRequest::Image(self.generate_request_to_domain(request).await?),
                    prompt,
                ))
            }
            GenerationWorkRequestDto::Stream(request) => {
                let (base, prompt) = self.prepare_prompt(request.base).await?;
                Ok((
                    GenerationWorkRequest::Stream(GenerateImageStreamRequest {
                        base: self.generate_request_to_domain(base).await?,
                        stream: stream_mode_to_domain(request.stream),
                    }),
                    prompt,
                ))
            }
        }
    }

    async fn generate_request_to_domain(
        &self,
        value: GenerateImageRequestDto,
    ) -> AppResult<GenerateImageRequest> {
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

    async fn prepare_prompt(
        &self,
        mut value: GenerateImageRequestDto,
    ) -> AppResult<(
        GenerateImageRequestDto,
        atelier_prompt_resources::CompiledPrompt,
    )> {
        let compiled = self
            .app
            .prompt_compiler
            .compile_generation_prompt(crate::prompt_preparation::compile_request(
                crate::prompt_preparation::generation_prompt(&value),
            ))
            .await?;
        let snapshot = atelier_prompt_resources::CompiledPrompt {
            expanded_prompt: compiled.prompt.clone(),
            trace: compiled
                .trace
                .main_prompt
                .clone()
                .ok_or_else(|| AppError::new("prompt_compile", "missing main prompt trace"))?,
        };
        value.main_preset_id = None;
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
        Ok((value, snapshot))
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
                region_to_replace: crate::input::ImageInputResolver::new(&self.app.resource_reader)
                    .resolve(inpaint.region_to_replace)
                    .await?,
            }),
            None => None,
        };
        Ok(Img2ImgRequest {
            image: crate::input::ImageInputResolver::new(&self.app.resource_reader)
                .resolve(value.image)
                .await?,
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
            resolved.push(
                crate::input::ImageInputResolver::new(&self.app.resource_reader)
                    .reference(reference)
                    .await?,
            );
        }
        Ok(Some(resolved))
    }

    async fn upsert_generation_history(
        &self,
        batch_id: &str,
        job_id: &str,
        update: GenerationHistoryUpdate,
    ) -> AppResult<RunHistoryRecord> {
        upsert_generation_history_record(&self.app.run_history, batch_id, job_id, update).await
    }

    async fn update_generation_history_status(
        &self,
        job_id: &str,
        status: RunHistoryStatus,
        last_error: Option<String>,
    ) -> AppResult<()> {
        let Some(existing) = self
            .app
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
}

fn generation_work_sample_count(work: &GenerationWorkRequestDto) -> u32 {
    let count = match work {
        GenerationWorkRequestDto::Image(request) => request.n_samples,
        GenerationWorkRequestDto::Stream(request) => request.base.n_samples,
    };
    count.max(1)
}
