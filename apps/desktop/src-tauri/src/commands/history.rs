use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::history::{
    DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
    DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
    GenerationHistoryBatchDetailDto, GenerationHistoryBatchRequestDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, RerunGenerationHistoryBatchRequestDto,
    RerunGenerationHistoryBatchResponseDto, RerunGenerationHistoryItemRequestDto,
    RerunGenerationHistoryItemResponseDto, RunHistoryPageDto, RunHistoryQueryDto,
};
use tauri::State;

#[tauri::command]
pub async fn query_run_history(
    state: State<'_, DesktopState>,
    request: RunHistoryQueryDto,
) -> CommandResult<RunHistoryPageDto> {
    state.host.query_run_history(request).await
}

#[tauri::command]
pub async fn query_generation_history(
    state: State<'_, DesktopState>,
    request: GenerationHistoryQueryDto,
) -> CommandResult<GenerationHistoryPageDto> {
    state.host.query_generation_history(request).await
}

#[tauri::command]
pub async fn get_generation_history_batch(
    state: State<'_, DesktopState>,
    request: GenerationHistoryBatchRequestDto,
) -> CommandResult<GenerationHistoryBatchDetailDto> {
    state.host.get_generation_history_batch(request).await
}

#[tauri::command]
pub async fn delete_run_history_items(
    state: State<'_, DesktopState>,
    request: DeleteRunHistoryItemsRequestDto,
) -> CommandResult<DeleteRunHistoryItemsResponseDto> {
    state.host.delete_run_history_items(request).await
}

#[tauri::command]
pub async fn delete_generation_history_batches(
    state: State<'_, DesktopState>,
    request: DeleteGenerationHistoryBatchesRequestDto,
) -> CommandResult<DeleteGenerationHistoryBatchesResponseDto> {
    state.host.delete_generation_history_batches(request).await
}

#[tauri::command]
pub async fn rerun_generation_history_item(
    state: State<'_, DesktopState>,
    request: RerunGenerationHistoryItemRequestDto,
) -> CommandResult<RerunGenerationHistoryItemResponseDto> {
    let response = state.host.rerun_generation_history_item(request).await?;
    state.kick_generation_worker(response.directive.clone());
    Ok(response)
}

#[tauri::command]
pub async fn rerun_generation_history_batch(
    state: State<'_, DesktopState>,
    request: RerunGenerationHistoryBatchRequestDto,
) -> CommandResult<RerunGenerationHistoryBatchResponseDto> {
    let response = state.host.rerun_generation_history_batch(request).await?;
    state.kick_generation_worker(response.directive.clone());
    Ok(response)
}
