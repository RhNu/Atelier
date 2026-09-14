use crate::desktop::DesktopState;
use atelier_app::CommandResult;
use atelier_app_api::generation::{
    GenerationAnlasEstimateDto, GenerationEstimateRequestDto, GenerationStatusDto,
    GenerationStatusQueryDto, QueueDirectiveDto, RunGenerationJobRequestDto,
    SaveGenerationDraftRequestDto, SubmitGenerationBatchRequestDto, SubmitGenerationRequestDto,
    VersionedGenerationDraftDto,
};
use atelier_app_api::prompt::{
    AppendLexiconEntitiesRequestDto, CompileGenerationPromptRequestDto, CompiledGenerationPromptDto,
};
use tauri::State;

#[tauri::command]
pub async fn compile_generation_prompt_preview(
    state: State<'_, DesktopState>,
    request: CompileGenerationPromptRequestDto,
) -> CommandResult<CompiledGenerationPromptDto> {
    state.host.compile_generation_prompt_preview(request).await
}

#[tauri::command]
pub async fn get_generation_draft(
    state: State<'_, DesktopState>,
) -> CommandResult<Option<VersionedGenerationDraftDto>> {
    state.host.get_generation_draft().await
}

#[tauri::command]
pub async fn save_generation_draft(
    state: State<'_, DesktopState>,
    request: SaveGenerationDraftRequestDto,
) -> CommandResult<VersionedGenerationDraftDto> {
    state.host.save_generation_draft(request).await
}

#[tauri::command]
pub async fn clear_generation_draft(state: State<'_, DesktopState>) -> CommandResult<()> {
    state.host.clear_generation_draft().await
}

#[tauri::command]
pub async fn append_lexicon_entities_to_generation_draft(
    state: State<'_, DesktopState>,
    request: AppendLexiconEntitiesRequestDto,
) -> CommandResult<VersionedGenerationDraftDto> {
    state
        .host
        .append_lexicon_entities_to_generation_draft(request)
        .await
}

#[tauri::command]
pub async fn submit_generation(
    state: State<'_, DesktopState>,
    request: SubmitGenerationRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.submit_generation(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn submit_generation_batch(
    state: State<'_, DesktopState>,
    request: SubmitGenerationBatchRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.submit_generation_batch(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn estimate_generation(
    state: State<'_, DesktopState>,
    request: GenerationEstimateRequestDto,
) -> CommandResult<GenerationAnlasEstimateDto> {
    state.host.estimate_generation(request).await
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands extract managed state by value"
)]
pub fn list_image_models(
    state: State<'_, DesktopState>,
) -> CommandResult<Vec<atelier_app_api::generation::ImageModelDescriptorDto>> {
    state.host.list_image_models()
}

#[tauri::command]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands extract managed state and request DTOs by value"
)]
pub async fn count_prompt_tokens(
    state: State<'_, DesktopState>,
    request: atelier_app_api::generation::CountPromptTokensRequestDto,
) -> CommandResult<atelier_app_api::generation::PromptTokenUsageDto> {
    state.host.count_prompt_tokens(request).await
}

#[tauri::command]
pub async fn run_generation_job(
    state: State<'_, DesktopState>,
    request: RunGenerationJobRequestDto,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.run_generation_job(request).await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn pause_generation_queue(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    state.cancel_generation_worker();
    state.host.pause_generation_queue().await
}

#[tauri::command]
pub async fn resume_generation_queue(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.resume_generation_queue().await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn stop_generation_queue(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    state.cancel_generation_worker_and_clear_pending();
    state.host.stop_generation_queue().await
}

#[tauri::command]
pub async fn generation_delay_elapsed(
    state: State<'_, DesktopState>,
) -> CommandResult<QueueDirectiveDto> {
    let directive = state.host.generation_delay_elapsed().await?;
    state.kick_generation_worker(directive.clone());
    Ok(directive)
}

#[tauri::command]
pub async fn generation_status(
    state: State<'_, DesktopState>,
    request: GenerationStatusQueryDto,
) -> CommandResult<GenerationStatusDto> {
    state.host.generation_status(request).await
}
