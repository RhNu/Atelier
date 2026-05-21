use super::{
    AppError, AppResult, AtelierApp, DirectorToolResultDto, ImageInputDto, NovelAiClientFactory,
    RunDirectorTool, RunDirectorToolRequest, RunDirectorToolRequestDto, RunHistoryKind,
    RunHistoryRecord, RunHistoryRepository, RunHistoryStatus, RunOutputRecord, SecretStore,
    SecretsErrorKind, director_tool_to_domain, gallery_item_to_dto, resource_ref_from_dto,
    resource_ref_to_dto, resource_variant_kind_as_str, unix_timestamp_ms, visual_asset_role_as_str,
};

pub struct DirectorUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
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
        let run_id = request.run_id.clone();
        let title = Some(format!("{:?}", request.tool).to_lowercase());
        let image = match self.image_input_to_base64(request.image).await {
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
        let mut kernel = self.app.inner.kernel.lock().await;
        let result = match kernel.run_director_tool(work).await {
            Ok(result) => result,
            Err(error) => {
                drop(kernel);
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
        drop(kernel);
        self.upsert_director_history(&run_id, title, RunHistoryStatus::Succeeded, None)
            .await?;
        for asset in &result.item.assets {
            self.app
                .inner
                .run_history
                .upsert_run_output(RunOutputRecord {
                    run_id: run_id.clone(),
                    artifact_id: result.artifact_id.as_str().to_owned(),
                    item_id: Some(result.item.id.as_str().to_owned()),
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
            .inner
            .run_history
            .get_run_history(run_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        self.app
            .inner
            .run_history
            .upsert_run_history(RunHistoryRecord {
                run_id: run_id.to_owned(),
                kind: RunHistoryKind::Director,
                status,
                batch_id: None,
                job_id: None,
                origin_run_id: None,
                submitted_payload_ref: None,
                prepared_payload_ref: None,
                title: title.or_else(|| existing.as_ref().and_then(|record| record.title.clone())),
                last_error,
                created_at_ms: existing.as_ref().map_or(now, |record| record.created_at_ms),
                updated_at_ms: now,
                completed_at_ms: Some(now),
                recoverable: false,
            })
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))
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
}
