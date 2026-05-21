use super::{
    AppError, AppResult, AtelierApp, CompilePromptRequest, CompilePromptRequestDto,
    CompiledPromptDto, DeletePromptChunkRequestDto, DeletePromptChunkResponseDto,
    GetPromptChunkRequestDto, ListPromptChunksRequestDto, PromptChunkDto, PromptChunkId,
    PromptChunkKey, PromptChunkPageDto, PromptLexiconCatalogDto, PromptLexiconListQueryDto,
    PromptLexiconPageDto, UpsertPromptChunkRequestDto, compiled_prompt_to_dto,
    lexicon_catalog_to_dto, lexicon_page_to_dto, lexicon_query_to_domain, lexicon_search_to_page,
    prompt_chunk_to_dto, upsert_prompt_chunk_to_domain,
};

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
