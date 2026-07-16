use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_app_api::history::{
    DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
    DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
    GenerationHistoryBatchDetailDto, GenerationHistoryBatchRequestDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, RerunGenerationHistoryBatchRequestDto,
    RerunGenerationHistoryBatchResponseDto, RerunGenerationHistoryItemRequestDto,
    RerunGenerationHistoryItemResponseDto, RunHistoryPageDto, RunHistoryQueryDto,
};
use atelier_jobs::{
    BatchId, JobId, JobPayloadRef, JobQueueSnapshot, RunHistoryKind, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus, RunOutputRecord,
};
use atelier_kernel::{
    GenerationPayloadStore, SubmitGenerationBatch, SubmitGenerationBatchJob, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
use atelier_secrets::{SecretStore, SecretsErrorKind};
use std::collections::BTreeSet;

mod persistence;
mod projection;

pub use persistence::{
    GenerationHistoryPosition, GenerationHistoryUpdate,
    generation_history_records_from_queue_snapshot, upsert_generation_history_record,
};

use persistence::ensure_generation_history_target_is_new;
use projection::{
    aggregate_generation_batch, generation_history_request_to_dto, preferred_run_outputs,
};

use crate::app::WorkspaceSession;
use crate::mapping::{
    generation_history_batch_to_dto, generation_history_page_to_dto,
    generation_history_query_to_domain, queue_directive_to_dto, run_history_item_to_dto,
    run_history_page_to_dto, run_history_query_to_domain, run_output_to_dto,
};
use crate::{AppError, AppResult};

pub struct HistoryUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}

impl<S, F, E> HistoryUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    pub async fn query(&self, query: RunHistoryQueryDto) -> AppResult<RunHistoryPageDto> {
        let domain_query = run_history_query_to_domain(&query);
        let records = self
            .app
            .inner
            .run_history
            .query_run_history(domain_query.clone())
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        let total = self
            .app
            .inner
            .run_history
            .count_run_history(domain_query)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        let mut items = Vec::with_capacity(records.len());
        for record in records {
            let outputs = self
                .app
                .inner
                .run_history
                .list_run_outputs(&record.run_id)
                .await
                .map_err(|error| AppError::new("run_history", error.to_string()))?;
            items.push(run_history_item_to_dto(record, outputs));
        }
        Ok(run_history_page_to_dto(
            items,
            query.offset,
            query.limit,
            total,
        ))
    }

    pub async fn query_generation(
        &self,
        query: GenerationHistoryQueryDto,
    ) -> AppResult<GenerationHistoryPageDto> {
        let domain_query = generation_history_query_to_domain(&query);
        let records = self
            .app
            .inner
            .run_history
            .query_generation_batches(domain_query)
            .await
            .map_err(history_error)?;
        let total = self
            .app
            .inner
            .run_history
            .count_generation_batches(generation_history_query_to_domain(&query))
            .await
            .map_err(history_error)?;
        let mut items = Vec::with_capacity(records.len());
        for record in records {
            let outputs = self.preferred_batch_outputs(&record.batch_id).await?;
            let completed_sample_count = outputs.len();
            items.push(generation_history_batch_to_dto(
                record,
                completed_sample_count,
                outputs.into_iter().take(4).map(run_output_to_dto).collect(),
            ));
        }
        Ok(generation_history_page_to_dto(
            items,
            query.offset,
            query.limit,
            total,
        ))
    }

    pub async fn get_generation_batch(
        &self,
        request: GenerationHistoryBatchRequestDto,
    ) -> AppResult<GenerationHistoryBatchDetailDto> {
        let records = self.generation_batch_records(&request.batch_id).await?;
        let mut requests = Vec::with_capacity(records.len());
        let mut batch_outputs = Vec::new();
        for (fallback_index, record) in records.iter().enumerate() {
            let outputs = self
                .app
                .inner
                .run_history
                .list_run_outputs(&record.run_id)
                .await
                .map_err(history_error)?;
            let preferred = preferred_run_outputs(outputs);
            batch_outputs.extend(preferred.iter().cloned());
            requests.push(generation_history_request_to_dto(
                record,
                fallback_index,
                preferred,
            ));
        }
        let aggregate = aggregate_generation_batch(&request.batch_id, &records);
        let completed_sample_count = batch_outputs.len();
        Ok(GenerationHistoryBatchDetailDto {
            batch: generation_history_batch_to_dto(
                aggregate,
                completed_sample_count,
                batch_outputs.into_iter().map(run_output_to_dto).collect(),
            ),
            requests,
        })
    }

    pub async fn delete_items(
        &self,
        request: DeleteRunHistoryItemsRequestDto,
    ) -> AppResult<DeleteRunHistoryItemsResponseDto> {
        let deleted = self
            .app
            .inner
            .run_history
            .delete_run_history_items(&request.run_ids)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?;
        Ok(DeleteRunHistoryItemsResponseDto { deleted })
    }

    pub async fn delete_generation_batches(
        &self,
        request: DeleteGenerationHistoryBatchesRequestDto,
    ) -> AppResult<DeleteGenerationHistoryBatchesResponseDto> {
        let deleted_requests = self
            .app
            .inner
            .run_history
            .delete_generation_batches(&request.batch_ids)
            .await
            .map_err(history_error)?;
        Ok(DeleteGenerationHistoryBatchesResponseDto { deleted_requests })
    }

    pub async fn rerun_generation(
        &self,
        request: RerunGenerationHistoryItemRequestDto,
    ) -> AppResult<RerunGenerationHistoryItemResponseDto> {
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
        let source = self
            .app
            .inner
            .run_history
            .get_run_history(&request.run_id)
            .await
            .map_err(|error| AppError::new("run_history", error.to_string()))?
            .ok_or_else(|| AppError::new("history_not_found", "run history item does not exist"))?;
        if source.kind != RunHistoryKind::Generation {
            return Err(AppError::new(
                "invalid_request",
                "only generation history items can be rerun",
            ));
        }
        self.ensure_rerun_target_is_new(&request).await?;
        let payload_ref = source
            .submitted_payload_ref
            .clone()
            .ok_or_else(|| AppError::new("history_not_found", "history item has no payload"))?;
        let submitted = {
            let kernel = self.app.inner.kernel.lock().await;
            kernel
                .ports()
                .get_submitted_payload(&JobPayloadRef::new(payload_ref))
                .await?
        }
        .ok_or_else(|| AppError::new("history_not_found", "submitted payload does not exist"))?;
        let title = submitted.request.prompt().to_owned();
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .submit_generation_work(SubmitGenerationWork {
                batch_id: BatchId::new(request.batch_id.clone()),
                job_id: JobId::new(request.job_id.clone()),
                request: submitted.request,
                context: submitted.context,
            })
            .await
            .map(queue_directive_to_dto)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        let persist_result = self.persist_queue_snapshot(&directive, &snapshot).await;
        if let Err(error) = persist_result {
            let _ = self
                .app
                .inner
                .kernel
                .lock()
                .await
                .restore_queue_snapshot(previous_snapshot);
            return Err(error);
        }
        let record = upsert_generation_history_record(
            &self.app.inner.run_history,
            &request.batch_id,
            &request.job_id,
            GenerationHistoryUpdate {
                status: RunHistoryStatus::Queued,
                title: Some(title),
                origin_run_id: Some(request.run_id),
                last_error: None,
                position: Some(GenerationHistoryPosition {
                    request_index: 0,
                    expected_samples: source.expected_samples.unwrap_or(1),
                }),
            },
        )
        .await?;
        Ok(RerunGenerationHistoryItemResponseDto {
            directive,
            item: run_history_item_to_dto(record, Vec::new()),
        })
    }

    pub async fn rerun_generation_batch(
        &self,
        request: RerunGenerationHistoryBatchRequestDto,
    ) -> AppResult<RerunGenerationHistoryBatchResponseDto> {
        self.ensure_active_api_key().await?;
        let sources = self
            .generation_batch_records(&request.source_batch_id)
            .await?;
        self.validate_generation_batch_rerun(&request, &sources)
            .await?;
        let submitted = self.load_generation_batch_payloads(&sources).await?;
        let context = submitted
            .first()
            .map(|payload| payload.context)
            .ok_or_else(|| AppError::new("history_not_found", "generation batch is empty"))?;
        if submitted.iter().any(|payload| payload.context != context) {
            return Err(AppError::new(
                "invalid_state",
                "source batch generation contexts do not match",
            ));
        }
        let jobs = request
            .job_ids
            .iter()
            .zip(submitted.into_iter())
            .map(|(job_id, payload)| SubmitGenerationBatchJob {
                job_id: JobId::new(job_id.clone()),
                request: payload.request,
            })
            .collect();
        let mut kernel = self.app.inner.kernel.lock().await;
        let previous_snapshot = kernel.queue_snapshot();
        let directive = kernel
            .submit_generation_batch(SubmitGenerationBatch {
                batch_id: BatchId::new(request.batch_id.clone()),
                jobs,
                context,
            })
            .await
            .map(queue_directive_to_dto)?;
        let snapshot = kernel.queue_snapshot();
        drop(kernel);
        if let Err(error) = self.persist_queue_snapshot(&directive, &snapshot).await {
            let _ = self
                .app
                .inner
                .kernel
                .lock()
                .await
                .restore_queue_snapshot(previous_snapshot);
            return Err(error);
        }

        let rerun_records = self
            .record_generation_batch_rerun(&request, &sources)
            .await?;
        let aggregate = aggregate_generation_batch(&request.batch_id, &rerun_records);
        Ok(RerunGenerationHistoryBatchResponseDto {
            directive,
            batch: generation_history_batch_to_dto(aggregate, 0, Vec::new()),
        })
    }

    async fn validate_generation_batch_rerun(
        &self,
        request: &RerunGenerationHistoryBatchRequestDto,
        sources: &[RunHistoryRecord],
    ) -> AppResult<()> {
        if sources.len() != request.job_ids.len() || request.job_ids.is_empty() {
            return Err(AppError::new(
                "invalid_request",
                "rerun job_ids must match the source batch request count",
            ));
        }
        if request.job_ids.iter().collect::<BTreeSet<_>>().len() != request.job_ids.len() {
            return Err(AppError::new(
                "invalid_request",
                "rerun job_ids must be unique",
            ));
        }
        for job_id in &request.job_ids {
            ensure_generation_history_target_is_new(
                &self.app.inner.run_history,
                &request.batch_id,
                job_id,
            )
            .await?;
        }
        Ok(())
    }

    async fn load_generation_batch_payloads(
        &self,
        sources: &[RunHistoryRecord],
    ) -> AppResult<Vec<SubmittedGenerationPayload>> {
        let kernel = self.app.inner.kernel.lock().await;
        let mut payloads = Vec::with_capacity(sources.len());
        for source in sources {
            let payload_ref = source.submitted_payload_ref.clone().ok_or_else(|| {
                AppError::new(
                    "history_not_found",
                    "history request has no submitted payload",
                )
            })?;
            let payload = kernel
                .ports()
                .get_submitted_payload(&JobPayloadRef::new(payload_ref))
                .await?
                .ok_or_else(|| {
                    AppError::new("history_not_found", "submitted payload does not exist")
                })?;
            payloads.push(payload);
        }
        drop(kernel);
        Ok(payloads)
    }

    async fn record_generation_batch_rerun(
        &self,
        request: &RerunGenerationHistoryBatchRequestDto,
        sources: &[RunHistoryRecord],
    ) -> AppResult<Vec<RunHistoryRecord>> {
        let mut records = Vec::with_capacity(sources.len());
        for (fallback_index, (source, job_id)) in
            sources.iter().zip(request.job_ids.iter()).enumerate()
        {
            records.push(
                upsert_generation_history_record(
                    &self.app.inner.run_history,
                    &request.batch_id,
                    job_id,
                    GenerationHistoryUpdate {
                        status: RunHistoryStatus::Queued,
                        title: source.title.clone(),
                        origin_run_id: Some(source.run_id.clone()),
                        last_error: None,
                        position: Some(GenerationHistoryPosition {
                            request_index: source.request_index.unwrap_or_else(|| {
                                u32::try_from(fallback_index).unwrap_or(u32::MAX)
                            }),
                            expected_samples: source.expected_samples.unwrap_or(1),
                        }),
                    },
                )
                .await?,
            );
        }
        Ok(records)
    }

    async fn preferred_batch_outputs(&self, batch_id: &str) -> AppResult<Vec<RunOutputRecord>> {
        let records = self.generation_batch_records(batch_id).await?;
        let mut outputs = Vec::new();
        for record in records {
            let run_outputs = self
                .app
                .inner
                .run_history
                .list_run_outputs(&record.run_id)
                .await
                .map_err(history_error)?;
            outputs.extend(preferred_run_outputs(run_outputs));
        }
        Ok(outputs)
    }

    async fn generation_batch_records(&self, batch_id: &str) -> AppResult<Vec<RunHistoryRecord>> {
        let records = self
            .app
            .inner
            .run_history
            .list_run_history_by_batch(batch_id)
            .await
            .map_err(history_error)?;
        if records.is_empty() {
            return Err(AppError::new(
                "history_not_found",
                "generation batch does not exist",
            ));
        }
        Ok(records)
    }

    async fn ensure_active_api_key(&self) -> AppResult<()> {
        self.app
            .inner
            .api_keys
            .resolve_active_secret()
            .await
            .map(|_| ())
            .map_err(|error| {
                if error.kind == SecretsErrorKind::MissingActiveKey {
                    AppError::missing_active_key()
                } else {
                    AppError::from(error)
                }
            })
    }

    async fn persist_queue_snapshot(
        &self,
        directive: &QueueDirectiveDto,
        snapshot: &JobQueueSnapshot,
    ) -> AppResult<()> {
        let history =
            generation_history_records_from_queue_snapshot(&self.app.inner.run_history, snapshot)
                .await?;
        let durable_snapshot = (!matches!(directive, QueueDirectiveDto::Idle)).then_some(snapshot);
        self.app
            .inner
            .queue_repository
            .commit_queue_and_history(durable_snapshot, history)
            .map_err(|error| AppError::new("job_queue", error.to_string()))
    }

    async fn ensure_rerun_target_is_new(
        &self,
        request: &RerunGenerationHistoryItemRequestDto,
    ) -> AppResult<()> {
        if request.job_id == request.run_id {
            return Err(AppError::new(
                "invalid_request",
                "rerun job_id must be different from the source run_id",
            ));
        }
        ensure_generation_history_target_is_new(
            &self.app.inner.run_history,
            &request.batch_id,
            &request.job_id,
        )
        .await
    }
}

fn history_error(error: impl std::fmt::Display) -> AppError {
    AppError::new("run_history", error.to_string())
}
