use atelier_app_api::prompt::{
    CompileGenerationPromptRequestDto, CompilePromptRequestDto, CompiledGenerationPromptDto,
    CompiledPromptDto, DeletePromptChunkRequestDto, DeletePromptChunkResponseDto,
    DeletePromptPresetRequestDto, DeletePromptPresetResponseDto, GetPromptChunkRequestDto,
    ListPromptChunksRequestDto, ListPromptPresetsRequestDto, PromptChunkDto, PromptChunkPageDto,
    PromptLexiconCatalogDto, PromptLexiconListQueryDto, PromptLexiconPageDto,
    PromptLexiconSearchQueryDto, PromptPresetDto, PromptPresetPageDto, UpsertPromptChunkRequestDto,
    UpsertPromptPresetRequestDto,
};

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Creates or updates a prompt chunk.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, validation fails, or prompt storage fails.
    pub async fn upsert_prompt_chunk(
        &self,
        request: UpsertPromptChunkRequestDto,
    ) -> CommandResult<PromptChunkDto> {
        Self::command_result(self.current_session()?.prompt().upsert_chunk(request).await)
    }

    /// Returns one prompt chunk by id or key.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the request is invalid, or the chunk is missing.
    pub async fn get_prompt_chunk(
        &self,
        request: GetPromptChunkRequestDto,
    ) -> CommandResult<PromptChunkDto> {
        Self::command_result(self.current_session()?.prompt().get_chunk(request).await)
    }

    /// Lists prompt chunks with offset and limit.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or prompt storage fails.
    pub async fn list_prompt_chunks(
        &self,
        request: ListPromptChunksRequestDto,
    ) -> CommandResult<PromptChunkPageDto> {
        Self::command_result(self.current_session()?.prompt().list_chunks(request).await)
    }

    /// Deletes an unreferenced prompt chunk.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the chunk is referenced, or prompt storage fails.
    pub async fn delete_prompt_chunk(
        &self,
        request: DeletePromptChunkRequestDto,
    ) -> CommandResult<DeletePromptChunkResponseDto> {
        Self::command_result(self.current_session()?.prompt().delete_chunk(request).await)
    }

    /// Creates or updates a prompt preset.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, validation fails, or prompt storage fails.
    pub async fn upsert_prompt_preset(
        &self,
        request: UpsertPromptPresetRequestDto,
    ) -> CommandResult<PromptPresetDto> {
        Self::command_result(
            self.current_session()?
                .prompt()
                .upsert_preset(request)
                .await,
        )
    }

    /// Lists prompt presets with offset and limit.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or prompt storage fails.
    pub async fn list_prompt_presets(
        &self,
        request: ListPromptPresetsRequestDto,
    ) -> CommandResult<PromptPresetPageDto> {
        Self::command_result(self.current_session()?.prompt().list_presets(request).await)
    }

    /// Deletes a prompt preset.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or prompt storage fails.
    pub async fn delete_prompt_preset(
        &self,
        request: DeletePromptPresetRequestDto,
    ) -> CommandResult<DeletePromptPresetResponseDto> {
        Self::command_result(
            self.current_session()?
                .prompt()
                .delete_preset(request)
                .await,
        )
    }

    /// Compiles a prompt preview using persisted prompt resources.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or prompt compilation fails.
    pub async fn compile_prompt_preview(
        &self,
        request: CompilePromptRequestDto,
    ) -> CommandResult<CompiledPromptDto> {
        Self::command_result(
            self.current_session()?
                .prompt()
                .compile_preview(request)
                .await,
        )
    }

    /// Compiles all Generation prompt scopes using persisted prompt resources.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or prompt compilation fails.
    pub async fn compile_generation_prompt_preview(
        &self,
        request: CompileGenerationPromptRequestDto,
    ) -> CommandResult<CompiledGenerationPromptDto> {
        Self::command_result(
            self.current_session()?
                .prompt()
                .compile_generation_preview(request)
                .await,
        )
    }

    /// Returns prompt lexicon catalog metadata.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open.
    pub fn prompt_lexicon_catalog(&self) -> CommandResult<PromptLexiconCatalogDto> {
        Ok(self.current_session()?.prompt().lexicon_catalog())
    }

    /// Lists prompt lexicon entries.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the lexicon query is invalid.
    pub fn prompt_lexicon_list(
        &self,
        request: PromptLexiconListQueryDto,
    ) -> CommandResult<PromptLexiconPageDto> {
        Self::command_result(self.current_session()?.prompt().lexicon_list(request))
    }

    /// Searches prompt lexicon entries.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the query limit is invalid.
    pub fn prompt_lexicon_search(
        &self,
        request: PromptLexiconSearchQueryDto,
    ) -> CommandResult<PromptLexiconPageDto> {
        let PromptLexiconSearchQueryDto { query, limit } = request;
        Self::command_result(
            self.current_session()?
                .prompt()
                .lexicon_search(&query, limit),
        )
    }
}
