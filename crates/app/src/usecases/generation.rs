use super::{
    AnlasEstimate, AppError, AppResult, ArtifactSource, AtelierApp, BatchId, CharacterReference,
    CharacterReferenceDto, ControlNetConfig, ControlNetConfigDto, ControlNetInput, GalleryQuery,
    GallerySourceKind, GenerateImageRequest, GenerateImageRequestDto, GenerateImageStreamRequest,
    GenerateImageStreamRequestDto, GenerationAnlasEstimateDto, GenerationEstimateRequestDto,
    GenerationStatusDto, GenerationWorkRequest, GenerationWorkRequestDto, ImageInputDto, ImageSize,
    Img2ImgRequest, Img2ImgRequestDto, JobId, JobQueueRepository, NovelAiClientFactory,
    QueueDirectiveDto, RunHistoryRecord, RunHistoryRepository, RunHistoryStatus, RunOutputRecord,
    SecretStore, SecretsErrorKind, SubmitGenerationBatch, SubmitGenerationBatchJob,
    SubmitGenerationBatchJobDto, SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto,
    character_reference_type_to_domain, characters_to_domain, generation_status_to_dto,
    generation_work_title, image_format_to_domain, image_model_to_domain, noise_schedule_to_domain,
    plan_context_to_domain, plan_generation_request, queue_directive_to_dto, resource_ref_from_dto,
    resource_variant_kind_as_str, run_history_status_from_job_status, sampler_to_domain,
    stream_mode_to_domain, sync_generation_history_from_queue_snapshot, uc_preset_to_domain,
    upsert_generation_history_record, visual_asset_role_as_str,
};

pub struct GenerationUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
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
        let titles = request
            .jobs
            .iter()
            .map(|job| (job.job_id.clone(), generation_work_title(&job.work)))
            .collect::<Vec<_>>();
        let work = self.submit_batch_request_to_domain(request).await?;
        let mut kernel = self.app.inner.kernel.lock().await;
        let directive = kernel
            .submit_generation_batch(work)
            .await
            .map(queue_directive_to_dto)
            .map_err(AppError::from)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        self.persist_queue_snapshot(&directive, &snapshot).await?;
        for (job_id, title) in titles {
            self.upsert_generation_history(
                &batch_id,
                &job_id,
                RunHistoryStatus::Queued,
                title,
                None,
                None,
            )
            .await?;
        }
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
            controlnet: self.optional_controlnet_to_domain(value.controlnet).await?,
            character_references: self
                .optional_character_references_to_domain(value.character_references)
                .await?,
            characters: value.characters.map(characters_to_domain),
            use_coords: value.use_coords,
            image_format: value.image_format.map(image_format_to_domain),
            strict_mode: value.strict_mode,
        })
    }

    async fn optional_controlnet_to_domain(
        &self,
        value: Option<ControlNetConfigDto>,
    ) -> AppResult<Option<ControlNetConfig>> {
        let Some(config) = value else {
            return Ok(None);
        };
        let mut images = Vec::with_capacity(config.images.len());
        for image in config.images {
            let reference = resource_ref_from_dto(image.encoding);
            let kernel = self.app.inner.kernel.lock().await;
            let vibe_data_cache = kernel
                .ports()
                .resource_reader
                .read_resource_base64(&reference)
                .await
                .map_err(AppError::from)?;
            drop(kernel);
            images.push(ControlNetInput {
                vibe_data_cache,
                info_extracted: image.info_extracted,
                strength: image.strength,
            });
        }
        Ok(Some(ControlNetConfig {
            images,
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
        snapshot: &atelier_jobs::JobQueueSnapshot,
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

async fn ensure_generation_batch_target_is_new<'a, R, I>(
    repository: &R,
    batch_id: &str,
    job_ids: I,
) -> AppResult<()>
where
    R: RunHistoryRepository,
    I: IntoIterator<Item = &'a str>,
{
    let job_ids = job_ids.into_iter().collect::<Vec<_>>();
    if job_ids.is_empty() {
        return Err(AppError::new(
            "invalid_request",
            "generation batch requires at least one job",
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    for job_id in &job_ids {
        if !unique.insert(*job_id) {
            return Err(AppError::new(
                "invalid_request",
                "generation batch contains duplicate job_id",
            ));
        }
    }
    if repository
        .run_history_batch_exists(batch_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?
    {
        return Err(AppError::new(
            "invalid_request",
            "generation batch_id already exists in run history",
        ));
    }
    for job_id in job_ids {
        if repository
            .get_run_history(job_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?
            .is_some()
        {
            return Err(AppError::new(
                "invalid_request",
                "generation job_id already exists in run history",
            ));
        }
    }
    Ok(())
}

const fn anlas_estimate_to_dto(value: AnlasEstimate) -> GenerationAnlasEstimateDto {
    GenerationAnlasEstimateDto {
        per_sample_cost: value.per_sample_cost,
        per_request_cost: value.per_request_cost,
        total_cost: value.total_cost,
        adjusted_resolution: value.adjusted_resolution,
        opus_discount_applied: value.opus_discount_applied,
        pending_encode_cost: value.pending_encode_cost,
    }
}

pub fn estimate_generation_anlas(
    request: &GenerationEstimateRequestDto,
) -> AppResult<GenerationAnlasEstimateDto> {
    let plan = plan_generation_request(
        estimate_request_to_domain(&request.request),
        plan_context_to_domain(request.context),
    )
    .map_err(|error| AppError::new("invalid_request", error.to_string()))?;
    Ok(anlas_estimate_to_dto(plan.anlas_estimate))
}

fn estimate_request_to_domain(value: &GenerateImageRequestDto) -> GenerateImageRequest {
    GenerateImageRequest {
        prompt: if value.prompt.trim().is_empty() {
            "estimate".to_owned()
        } else {
            value.prompt.clone()
        },
        model: image_model_to_domain(value.model),
        size: ImageSize {
            width: value.size.width,
            height: value.size.height,
        },
        negative_prompt: value.negative_prompt.clone(),
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
        i2i: value.i2i.as_ref().map(|i2i| Img2ImgRequest {
            image: "estimate".to_owned(),
            strength: i2i.strength,
            noise: i2i.noise,
            mask: i2i.mask.as_ref().map(|_| "estimate".to_owned()),
        }),
        controlnet: value
            .controlnet
            .as_ref()
            .map(|controlnet| ControlNetConfig {
                images: controlnet
                    .images
                    .iter()
                    .map(|image| ControlNetInput {
                        vibe_data_cache: "estimate".to_owned(),
                        info_extracted: image.info_extracted,
                        strength: image.strength,
                    })
                    .collect(),
                strength: controlnet.strength,
            }),
        character_references: value.character_references.as_ref().map(|references| {
            references
                .iter()
                .map(|reference| CharacterReference {
                    image: "estimate".to_owned(),
                    reference_type: character_reference_type_to_domain(reference.reference_type),
                    fidelity: reference.fidelity,
                    strength: reference.strength,
                })
                .collect()
        }),
        characters: value.characters.clone().map(characters_to_domain),
        use_coords: value.use_coords,
        image_format: value.image_format.map(image_format_to_domain),
        strict_mode: value.strict_mode,
    }
}
