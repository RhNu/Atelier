use super::join_error;
use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::prompt::{
    CompilePromptRequestDto, CompiledPromptDto, DeletePromptChunkRequestDto,
    DeletePromptChunkResponseDto, DeletePromptPresetRequestDto, DeletePromptPresetResponseDto,
    GetPromptChunkRequestDto, LexiconBootstrapDto, LexiconCompleteRequestDto,
    LexiconEntityDetailDto, LexiconEntityRequestDto, LexiconSearchItemDto, LexiconSearchPageDto,
    LexiconSearchRequestDto, ListPromptChunksRequestDto, ListPromptPresetsRequestDto,
    PromptChunkDto, PromptChunkPageDto, PromptPresetDto, PromptPresetPageDto,
    UpsertPromptChunkRequestDto, UpsertPromptPresetRequestDto,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn upsert_prompt_chunk(
    state: State<'_, DesktopState>,
    request: UpsertPromptChunkRequestDto,
) -> CommandResult<PromptChunkDto> {
    state.host.upsert_prompt_chunk(request).await
}

#[tauri::command]
pub async fn get_prompt_chunk(
    state: State<'_, DesktopState>,
    request: GetPromptChunkRequestDto,
) -> CommandResult<PromptChunkDto> {
    state.host.get_prompt_chunk(request).await
}

#[tauri::command]
pub async fn list_prompt_chunks(
    state: State<'_, DesktopState>,
    request: ListPromptChunksRequestDto,
) -> CommandResult<PromptChunkPageDto> {
    state.host.list_prompt_chunks(request).await
}

#[tauri::command]
pub async fn delete_prompt_chunk(
    state: State<'_, DesktopState>,
    request: DeletePromptChunkRequestDto,
) -> CommandResult<DeletePromptChunkResponseDto> {
    state.host.delete_prompt_chunk(request).await
}

#[tauri::command]
pub async fn upsert_prompt_preset(
    state: State<'_, DesktopState>,
    request: UpsertPromptPresetRequestDto,
) -> CommandResult<PromptPresetDto> {
    state.host.upsert_prompt_preset(request).await
}

#[tauri::command]
pub async fn list_prompt_presets(
    state: State<'_, DesktopState>,
    request: ListPromptPresetsRequestDto,
) -> CommandResult<PromptPresetPageDto> {
    state.host.list_prompt_presets(request).await
}

#[tauri::command]
pub async fn delete_prompt_preset(
    state: State<'_, DesktopState>,
    request: DeletePromptPresetRequestDto,
) -> CommandResult<DeletePromptPresetResponseDto> {
    state.host.delete_prompt_preset(request).await
}

#[tauri::command]
pub async fn compile_prompt_preview(
    state: State<'_, DesktopState>,
    request: CompilePromptRequestDto,
) -> CommandResult<CompiledPromptDto> {
    state.host.compile_prompt_preview(request).await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub async fn lexicon_bootstrap(
    state: State<'_, DesktopState>,
) -> CommandResult<LexiconBootstrapDto> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_bootstrap())
        .await
        .map_err(join_error)?
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub async fn lexicon_complete(
    state: State<'_, DesktopState>,
    request: LexiconCompleteRequestDto,
) -> CommandResult<Vec<LexiconSearchItemDto>> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_complete(&request))
        .await
        .map_err(join_error)?
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands inject State by value"
)]
pub async fn lexicon_search(
    state: State<'_, DesktopState>,
    request: LexiconSearchRequestDto,
) -> CommandResult<LexiconSearchPageDto> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_search(request))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn lexicon_entity(
    state: State<'_, DesktopState>,
    request: LexiconEntityRequestDto,
) -> CommandResult<LexiconEntityDetailDto> {
    let host = Arc::clone(&state.host);
    tauri::async_runtime::spawn_blocking(move || host.lexicon_entity(&request))
        .await
        .map_err(join_error)?
}
