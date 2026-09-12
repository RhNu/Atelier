use crate::mapping::{director_tool_to_domain, gallery_item_to_dto, resource_ref_to_dto};
use crate::session::WorkspaceSession;
use crate::time::unix_timestamp_ms;
use crate::{AppError, AppResult};
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::director::{DirectorToolResultDto, RunDirectorToolRequestDto};
use atelier_director::RunDirectorToolRequest;
use atelier_jobs::{RunHistoryKind, RunHistoryRecord, RunHistoryRepository, RunHistoryStatus};
use atelier_kernel::RunDirectorTool;
use atelier_secrets::{SecretStore, SecretsErrorKind};

pub struct DirectorUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}

impl<S, F, E> DirectorUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: Send + Sync,
{
    pub async fn run_tool(
        &self,
        request: RunDirectorToolRequestDto,
    ) -> AppResult<DirectorToolResultDto> {
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
        let run_id = request.run_id.clone();
        let title = Some(format!("{:?}", request.tool).to_lowercase());
        let image = match crate::input::ImageInputResolver::new(&self.app.resource_reader)
            .resolve(request.image)
            .await
        {
            Ok(image) => image,
            Err(error) => {
                self.upsert_director_history(
                    &run_id,
                    title.clone(),
                    RunHistoryStatus::Failed,
                    Some(error.to_string()),
                )
                .await?;
                return Err(error);
            }
        };
        let work = RunDirectorTool {
            run_id: request.run_id,
            request: RunDirectorToolRequest {
                tool: director_tool_to_domain(request.tool),
                image,
                prompt: request.prompt,
                defry: request.defry,
                strict_mode: request.strict_mode,
            },
        };
        self.upsert_director_history(&run_id, title.clone(), RunHistoryStatus::Running, None)
            .await?;
        let kernel = &self.app.workflows;
        let result = match kernel.run_director_tool(work).await {
            Ok(result) => result,
            Err(error) => {
                let app_error = AppError::from(error);
                self.upsert_director_history(
                    &run_id,
                    title,
                    RunHistoryStatus::Failed,
                    Some(app_error.to_string()),
                )
                .await?;
                return Err(app_error);
            }
        };
        self.upsert_director_history(&run_id, title, RunHistoryStatus::Succeeded, None)
            .await?;
        Ok(DirectorToolResultDto {
            item_id: result.item.id.as_str().to_owned(),
            artifact_id: result.artifact_id.as_str().to_owned(),
            resource: resource_ref_to_dto(&result.resource),
            item: gallery_item_to_dto(result.item),
        })
    }

    async fn upsert_director_history(
        &self,
        run_id: &str,
        title: Option<String>,
        status: RunHistoryStatus,
        last_error: Option<String>,
    ) -> AppResult<()> {
        let now = unix_timestamp_ms();
        let existing = self
            .app
            .run_history
            .get_run_history(run_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        self.app
            .run_history
            .upsert_run_history(RunHistoryRecord {
                run_id: run_id.to_owned(),
                kind: RunHistoryKind::Director,
                status,
                batch_id: None,
                job_id: None,
                origin_run_id: None,
                request_index: None,
                expected_samples: None,
                submitted_payload_ref: None,
                prepared_payload_ref: None,
                title: title.or_else(|| existing.as_ref().and_then(|record| record.title.clone())),
                last_error,
                created_at_ms: existing.as_ref().map_or(now, |record| record.created_at_ms),
                updated_at_ms: now,
                completed_at_ms: (!matches!(status, RunHistoryStatus::Running)).then_some(now),
                recoverable: false,
            })
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))
    }
}
