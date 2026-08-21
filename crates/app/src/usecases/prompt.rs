use super::{
    AppError, AppResult, CompileCharacterPromptRequest, CompileGenerationPromptRequest,
    CompileGenerationPromptRequestDto, CompilePromptRequest, CompilePromptRequestDto,
    CompiledGenerationCharacterPromptDto, CompiledGenerationPromptDto, CompiledPromptDto,
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, DeletePromptPresetRequestDto,
    DeletePromptPresetResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
    ListPromptPresetsRequestDto, PromptChunkDto, PromptChunkId, PromptChunkKey, PromptChunkPageDto,
    PromptPresetDto, PromptPresetId, PromptPresetPageDto, UpsertPromptChunkRequestDto,
    UpsertPromptPresetRequestDto, WorkspaceSession, compiled_prompt_to_dto, image_model_to_domain,
    prompt_chunk_to_dto, prompt_preset_kind_to_domain, prompt_preset_to_dto, prompt_trace_to_dto,
    quality_preset_to_dto, upsert_prompt_chunk_to_domain, upsert_prompt_preset_to_domain,
};
use atelier_resource_catalog::{
    ResourceCatalogErrorKind, ResourceId, ResourceOwner, ResourceOwnerKind, ResourceRef,
    ResourceRelation,
};

pub struct PromptUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
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
        let _write_guard = self.app.inner.prompt_resource_write.lock().await;
        let request = upsert_prompt_chunk_to_domain(request)?;
        let existing = if let Some(id) = &request.chunk_id {
            self.app.inner.prompt_chunks.get_chunk_by_id(id).await?
        } else {
            None
        };
        let requested_preview = request.preview_thumb.clone();
        let existing_owner = request.chunk_id.as_ref().map(prompt_chunk_owner);
        let pending_owner = request.chunk_id.is_none().then(|| {
            pending_prompt_preview_owner(
                "chunk",
                requested_preview
                    .as_ref()
                    .map_or("none", |preview| preview.id.as_str()),
            )
        });
        let pre_save_owner = existing_owner.as_ref().or(pending_owner.as_ref());
        if let (Some(preview), Some(owner)) = (&requested_preview, pre_save_owner) {
            self.app
                .inner
                .resources
                .attach_owner(&preview.id, owner.clone(), ResourceRelation::Thumbnail)
                .await?;
        }
        let chunk = match self.app.inner.prompt_chunks.upsert_chunk(request).await {
            Ok(chunk) => chunk,
            Err(error) => {
                if let (Some(preview), Some(owner)) = (requested_preview.as_ref(), pre_save_owner)
                    && existing
                        .as_ref()
                        .and_then(|item| item.preview_thumb.as_ref())
                        .is_none_or(|previous| previous.id != preview.id)
                {
                    let _ = detach_prompt_preview(self.app, preview, owner).await;
                    let _ = self.app.inner.resources.cleanup_delete_pending().await;
                }
                return Err(error.into());
            }
        };
        let owner = prompt_chunk_owner(&chunk.id);
        reconcile_prompt_preview(
            self.app,
            existing
                .as_ref()
                .and_then(|item| item.preview_thumb.as_ref()),
            chunk.preview_thumb.as_ref(),
            &owner,
            pending_owner.as_ref(),
        )
        .await?;
        Ok(prompt_chunk_to_dto(&chunk))
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
        let chunks = self
            .app
            .inner
            .prompt_chunks
            .list_chunks(request.model.map(image_model_to_domain))
            .await?;
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
        let _write_guard = self.app.inner.prompt_resource_write.lock().await;
        let id = PromptChunkId::new(request.chunk_id);
        let existing = self.app.inner.prompt_chunks.get_chunk_by_id(&id).await?;
        let result = self.app.inner.prompt_chunks.delete_chunk(&id).await?;
        if result.deleted
            && let Some(preview) = existing.and_then(|item| item.preview_thumb)
        {
            detach_prompt_preview(self.app, &preview, &prompt_chunk_owner(&id)).await?;
            self.app.inner.resources.cleanup_delete_pending().await?;
        }
        Ok(DeletePromptChunkResponseDto {
            deleted: result.deleted,
        })
    }

    pub async fn upsert_preset(
        &self,
        request: UpsertPromptPresetRequestDto,
    ) -> AppResult<PromptPresetDto> {
        let _write_guard = self.app.inner.prompt_resource_write.lock().await;
        let request = upsert_prompt_preset_to_domain(request);
        let existing = if let Some(id) = &request.preset_id {
            self.app.inner.prompt_presets.get_preset_by_id(id).await?
        } else {
            None
        };
        let requested_preview = request.preview_thumb.clone();
        let existing_owner = request.preset_id.as_ref().map(prompt_preset_owner);
        let pending_owner = request.preset_id.is_none().then(|| {
            pending_prompt_preview_owner(
                "preset",
                requested_preview
                    .as_ref()
                    .map_or("none", |preview| preview.id.as_str()),
            )
        });
        let pre_save_owner = existing_owner.as_ref().or(pending_owner.as_ref());
        if let (Some(preview), Some(owner)) = (&requested_preview, pre_save_owner) {
            self.app
                .inner
                .resources
                .attach_owner(&preview.id, owner.clone(), ResourceRelation::Thumbnail)
                .await?;
        }
        let preset = match self.app.inner.prompt_presets.upsert_preset(request).await {
            Ok(preset) => preset,
            Err(error) => {
                if let (Some(preview), Some(owner)) = (requested_preview.as_ref(), pre_save_owner)
                    && existing
                        .as_ref()
                        .and_then(|item| item.preview_thumb.as_ref())
                        .is_none_or(|previous| previous.id != preview.id)
                {
                    let _ = detach_prompt_preview(self.app, preview, owner).await;
                    let _ = self.app.inner.resources.cleanup_delete_pending().await;
                }
                return Err(error.into());
            }
        };
        let owner = prompt_preset_owner(&preset.id);
        reconcile_prompt_preview(
            self.app,
            existing
                .as_ref()
                .and_then(|item| item.preview_thumb.as_ref()),
            preset.preview_thumb.as_ref(),
            &owner,
            pending_owner.as_ref(),
        )
        .await?;
        Ok(prompt_preset_to_dto(&preset))
    }

    pub async fn list_presets(
        &self,
        request: ListPromptPresetsRequestDto,
    ) -> AppResult<PromptPresetPageDto> {
        let presets = self
            .app
            .inner
            .prompt_presets
            .list_presets(
                request.kind.map(prompt_preset_kind_to_domain),
                request.model.map(image_model_to_domain),
            )
            .await?;
        let total = presets.len();
        let start = request.offset.min(total);
        let end = start.saturating_add(request.limit).min(total);
        Ok(PromptPresetPageDto {
            items: presets[start..end]
                .iter()
                .map(prompt_preset_to_dto)
                .collect(),
            total,
            offset: request.offset,
            limit: request.limit,
        })
    }

    pub async fn delete_preset(
        &self,
        request: DeletePromptPresetRequestDto,
    ) -> AppResult<DeletePromptPresetResponseDto> {
        let _write_guard = self.app.inner.prompt_resource_write.lock().await;
        let id = PromptPresetId::new(request.preset_id);
        let existing = self.app.inner.prompt_presets.get_preset_by_id(&id).await?;
        let result = self.app.inner.prompt_presets.delete_preset(&id).await?;
        if result.deleted
            && let Some(preview) = existing.and_then(|item| item.preview_thumb)
        {
            detach_prompt_preview(self.app, &preview, &prompt_preset_owner(&id)).await?;
            self.app.inner.resources.cleanup_delete_pending().await?;
        }
        Ok(DeletePromptPresetResponseDto {
            deleted: result.deleted,
        })
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
                model: image_model_to_domain(request.model),
                max_depth: request.max_depth,
            })
            .await
            .map(|compiled| compiled_prompt_to_dto(&compiled))
            .map_err(AppError::from)
    }

    pub async fn compile_generation_preview(
        &self,
        request: CompileGenerationPromptRequestDto,
    ) -> AppResult<CompiledGenerationPromptDto> {
        let enabled_flags = request
            .characters
            .iter()
            .map(|character| character.enabled)
            .collect::<Vec<_>>();
        let max_depth = request.max_depth;
        let compiled = self
            .app
            .inner
            .prompt_compiler
            .compile_generation_prompt(CompileGenerationPromptRequest {
                model: image_model_to_domain(request.model),
                main_preset_id: request.main_preset_id.map(PromptPresetId::new),
                prompt: request.prompt,
                negative_prompt: request.negative_prompt.unwrap_or_default(),
                characters: request
                    .characters
                    .into_iter()
                    .enumerate()
                    .map(|(index, character)| CompileCharacterPromptRequest {
                        character_index: u32::try_from(index).unwrap_or(u32::MAX),
                        preset_id: character.preset_id.map(PromptPresetId::new),
                        prompt: character.prompt,
                        negative_prompt: character.negative_prompt.unwrap_or_default(),
                    })
                    .collect(),
                max_depth,
            })
            .await?;
        let prompt_trace = compiled
            .trace
            .main_prompt
            .as_ref()
            .ok_or_else(|| AppError::new("prompt_compile", "missing main prompt trace"))?;
        let prompt = CompiledPromptDto {
            expanded_prompt: compiled.prompt.clone(),
            trace: prompt_trace_to_dto(prompt_trace),
        };
        let negative_prompt = if compiled.negative_prompt.trim().is_empty() {
            None
        } else {
            let negative_trace = compiled
                .trace
                .main_negative_prompt
                .as_ref()
                .ok_or_else(|| AppError::new("prompt_compile", "missing negative prompt trace"))?;
            Some(CompiledPromptDto {
                expanded_prompt: compiled.negative_prompt.clone(),
                trace: prompt_trace_to_dto(negative_trace),
            })
        };
        let mut characters = Vec::with_capacity(enabled_flags.len());
        for (index, enabled) in enabled_flags.into_iter().enumerate() {
            let compiled_character = compiled.characters.iter().find(|character| {
                character.character_index == u32::try_from(index).unwrap_or(u32::MAX)
            });
            if let Some(compiled_character) = compiled_character {
                characters.push(CompiledGenerationCharacterPromptDto {
                    prompt: CompiledPromptDto {
                        expanded_prompt: compiled_character.prompt.clone(),
                        trace: prompt_trace_to_dto(&compiled_character.trace),
                    },
                    negative_prompt: (!compiled_character.negative_prompt.trim().is_empty()).then(
                        || CompiledPromptDto {
                            expanded_prompt: compiled_character.negative_prompt.clone(),
                            trace: prompt_trace_to_dto(&compiled_character.negative_trace),
                        },
                    ),
                    enabled,
                });
            } else {
                let empty = self
                    .compile_preview(CompilePromptRequestDto {
                        prompt: String::new(),
                        model: request.model,
                        max_depth,
                    })
                    .await?;
                characters.push(CompiledGenerationCharacterPromptDto {
                    prompt: empty,
                    negative_prompt: None,
                    enabled,
                });
            }
        }
        Ok(CompiledGenerationPromptDto {
            prompt,
            negative_prompt,
            characters,
            quality_override: compiled.quality_override.map(quality_preset_to_dto),
            uc_preset_override: compiled.uc_preset_override,
        })
    }
}

fn prompt_chunk_owner(id: &PromptChunkId) -> ResourceOwner {
    ResourceOwner::new(
        ResourceOwnerKind::PromptResource,
        format!("chunk:{}", id.as_str()),
    )
}

fn prompt_preset_owner(id: &PromptPresetId) -> ResourceOwner {
    ResourceOwner::new(
        ResourceOwnerKind::PromptResource,
        format!("preset:{}", id.as_str()),
    )
}

fn pending_prompt_preview_owner(kind: &str, resource_id: &str) -> ResourceOwner {
    ResourceOwner::new(
        ResourceOwnerKind::PromptResource,
        format!("pending:{kind}:{resource_id}"),
    )
}

async fn reconcile_prompt_preview<S, F, E>(
    app: &WorkspaceSession<S, F, E>,
    previous: Option<&ResourceRef>,
    current: Option<&ResourceRef>,
    owner: &ResourceOwner,
    pending_owner: Option<&ResourceOwner>,
) -> AppResult<()>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    if let Some(current) = current {
        app.inner
            .resources
            .attach_owner(&current.id, owner.clone(), ResourceRelation::Thumbnail)
            .await?;
    }
    if let (Some(current), Some(pending_owner)) = (current, pending_owner) {
        detach_prompt_preview(app, current, pending_owner).await?;
    }
    if let Some(previous) = previous
        && current.is_none_or(|current| current.id != previous.id)
    {
        detach_prompt_preview(app, previous, owner).await?;
    }
    if let Some(current) = current {
        release_import_staging_preview(app, &current.id).await?;
    }
    app.inner.resources.cleanup_delete_pending().await?;
    Ok(())
}

async fn release_import_staging_preview<S, F, E>(
    app: &WorkspaceSession<S, F, E>,
    resource_id: &ResourceId,
) -> AppResult<()>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    let staging_owner = ResourceOwner::new(ResourceOwnerKind::ImportStaging, "user-image-inputs");
    for link in app
        .inner
        .resources
        .list_links_by_owner(&staging_owner)
        .await?
    {
        if link.resource_id == *resource_id {
            app.inner
                .resources
                .detach_owner(&link.resource_id, &staging_owner, link.relation)
                .await?;
        }
    }
    Ok(())
}

async fn detach_prompt_preview<S, F, E>(
    app: &WorkspaceSession<S, F, E>,
    preview: &ResourceRef,
    owner: &ResourceOwner,
) -> AppResult<()>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    match app
        .inner
        .resources
        .detach_owner(&preview.id, owner, ResourceRelation::Thumbnail)
        .await
    {
        Ok(_) => Ok(()),
        Err(error) if error.kind == ResourceCatalogErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
