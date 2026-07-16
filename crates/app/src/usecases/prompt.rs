use super::{
    AppError, AppResult, CompileCharacterPromptRequest, CompileGenerationPromptRequest,
    CompileGenerationPromptRequestDto, CompilePromptRequest, CompilePromptRequestDto,
    CompiledGenerationCharacterPromptDto, CompiledGenerationPromptDto, CompiledPromptDto,
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, DeletePromptPresetRequestDto,
    DeletePromptPresetResponseDto, GetPromptChunkRequestDto, ListPromptChunksRequestDto,
    ListPromptPresetsRequestDto, PromptChunkDto, PromptChunkId, PromptChunkKey, PromptChunkPageDto,
    PromptLexiconCatalogDto, PromptLexiconListQueryDto, PromptLexiconPageDto, PromptPresetDto,
    PromptPresetId, PromptPresetPageDto, UpsertPromptChunkRequestDto, UpsertPromptPresetRequestDto,
    WorkspaceSession, compiled_prompt_to_dto, lexicon_catalog_to_dto, lexicon_page_to_dto,
    lexicon_query_to_domain, lexicon_search_to_page, prompt_chunk_to_dto,
    prompt_preset_kind_to_domain, prompt_preset_to_dto, prompt_trace_to_dto,
    upsert_prompt_chunk_to_domain, upsert_prompt_preset_to_domain,
};
use atelier_prompt::{FunctionRegistry, PromptDiagnosticKind, PromptSyntaxProfile, parse_prompt};

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

    pub async fn upsert_preset(
        &self,
        request: UpsertPromptPresetRequestDto,
    ) -> AppResult<PromptPresetDto> {
        self.app
            .inner
            .prompt_presets
            .upsert_preset(upsert_prompt_preset_to_domain(request))
            .await
            .map(|preset| prompt_preset_to_dto(&preset))
            .map_err(AppError::from)
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
                request.include_disabled,
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
        self.app
            .inner
            .prompt_presets
            .delete_preset(&PromptPresetId::new(request.preset_id))
            .await
            .map(|result| DeletePromptPresetResponseDto {
                deleted: result.deleted,
            })
            .map_err(AppError::from)
    }

    pub async fn compile_preview(
        &self,
        request: CompilePromptRequestDto,
    ) -> AppResult<CompiledPromptDto> {
        validate_prompt_syntax(&request.prompt)?;
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

    pub async fn compile_generation_preview(
        &self,
        request: CompileGenerationPromptRequestDto,
    ) -> AppResult<CompiledGenerationPromptDto> {
        validate_generation_prompt_syntax(&request)?;
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
            quality_override: compiled.quality_override,
            uc_preset_override: compiled.uc_preset_override,
        })
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

fn validate_prompt_syntax(prompt: &str) -> AppResult<()> {
    let diagnostics = parse_prompt(prompt).diagnostics_with_functions(
        &PromptSyntaxProfile::novelai_v45(),
        &FunctionRegistry::atelier_defaults(),
    );
    if let Some(diagnostic) = diagnostics.into_iter().find(|item| {
        !matches!(
            item.kind,
            PromptDiagnosticKind::UnclosedStrengthening
                | PromptDiagnosticKind::UnclosedWeakening
                | PromptDiagnosticKind::UnclosedNumericEmphasis
                | PromptDiagnosticKind::UnclosedRandomizer
                | PromptDiagnosticKind::UnclosedFunctionCall
        )
    }) {
        return Err(AppError::new(
            "prompt_syntax",
            format!("{} at byte {}", diagnostic.message, diagnostic.span.start),
        ));
    }
    Ok(())
}

fn validate_generation_prompt_syntax(request: &CompileGenerationPromptRequestDto) -> AppResult<()> {
    validate_prompt_syntax(&request.prompt)?;
    if let Some(negative_prompt) = &request.negative_prompt {
        validate_prompt_syntax(negative_prompt)?;
    }
    for character in &request.characters {
        validate_prompt_syntax(&character.prompt)?;
        if let Some(negative_prompt) = &character.negative_prompt {
            validate_prompt_syntax(negative_prompt)?;
        }
    }
    Ok(())
}
