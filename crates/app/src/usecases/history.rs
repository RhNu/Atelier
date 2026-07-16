use atelier_adapter_database::DatabaseRunHistoryRepository;
use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::generation::QueueDirectiveDto;
use atelier_app_api::history::{
    DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
    DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
    GenerationHistoryBatchDetailDto, GenerationHistoryBatchRequestDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, GenerationHistoryRequestDto, RerunGenerationHistoryBatchRequestDto,
    RerunGenerationHistoryBatchResponseDto, RerunGenerationHistoryItemRequestDto,
    RerunGenerationHistoryItemResponseDto, RunHistoryPageDto, RunHistoryQueryDto,
};
use atelier_jobs::{
    BatchId, BatchStatus, GenerationBatchHistoryRecord, GenerationBatchHistoryStatus, JobId,
    JobKind, JobPayloadRef, JobQueueSnapshot, JobRecord, JobStatus, RunHistoryKind,
    RunHistoryRecord, RunHistoryRepository, RunHistoryStatus, RunOutputRecord,
};
use atelier_kernel::{
    GenerationPayloadStore, SubmitGenerationBatch, SubmitGenerationBatchJob, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
use atelier_secrets::{SecretStore, SecretsErrorKind};
use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::WorkspaceSession;
use crate::mapping::{
    generation_history_batch_to_dto, generation_history_page_to_dto,
    generation_history_query_to_domain, queue_directive_to_dto, run_history_item_to_dto,
    run_history_page_to_dto, run_history_query_to_domain, run_history_status_to_dto,
    run_output_to_dto,
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

fn generation_history_request_to_dto(
    record: &RunHistoryRecord,
    fallback_index: usize,
    outputs: Vec<RunOutputRecord>,
) -> GenerationHistoryRequestDto {
    GenerationHistoryRequestDto {
        run_id: record.run_id.clone(),
        job_id: record
            .job_id
            .clone()
            .unwrap_or_else(|| record.run_id.clone()),
        origin_run_id: record.origin_run_id.clone(),
        request_index: record
            .request_index
            .unwrap_or_else(|| u32::try_from(fallback_index).unwrap_or(u32::MAX)),
        expected_samples: record.expected_samples.unwrap_or(1).max(1),
        status: run_history_status_to_dto(record.status),
        title: record.title.clone(),
        last_error: record.last_error.clone(),
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
        completed_at_ms: record.completed_at_ms,
        outputs: outputs.into_iter().map(run_output_to_dto).collect(),
    }
}

fn preferred_run_outputs(outputs: Vec<RunOutputRecord>) -> Vec<RunOutputRecord> {
    let mut seen_artifacts = BTreeSet::new();
    let mut seen_samples = BTreeSet::new();
    let mut next_sample = 0_u32;
    let mut preferred = Vec::new();
    for mut output in outputs {
        if !seen_artifacts.insert(output.artifact_id.clone()) {
            continue;
        }
        let sample_index = output.sample_index.unwrap_or_else(|| {
            while seen_samples.contains(&next_sample) {
                next_sample = next_sample.saturating_add(1);
            }
            next_sample
        });
        if !seen_samples.insert(sample_index) {
            continue;
        }
        output.sample_index = Some(sample_index);
        next_sample = next_sample.max(sample_index.saturating_add(1));
        preferred.push(output);
    }
    preferred.sort_by_key(|output| output.sample_index.unwrap_or(u32::MAX));
    preferred
}

fn aggregate_generation_batch(
    batch_id: &str,
    records: &[RunHistoryRecord],
) -> GenerationBatchHistoryRecord {
    let request_count = records.len();
    let completed_request_count = records
        .iter()
        .filter(|record| status_is_terminal(record.status))
        .count();
    let expected_sample_count = records.iter().fold(0_u32, |total, record| {
        total.saturating_add(record.expected_samples.unwrap_or(1).max(1))
    });
    GenerationBatchHistoryRecord {
        batch_id: batch_id.to_owned(),
        status: aggregate_generation_batch_status(records),
        title: records.first().and_then(|record| record.title.clone()),
        last_error: records
            .iter()
            .filter(|record| record.last_error.is_some())
            .max_by_key(|record| record.updated_at_ms)
            .and_then(|record| record.last_error.clone()),
        created_at_ms: records
            .iter()
            .map(|record| record.created_at_ms)
            .min()
            .unwrap_or(0),
        updated_at_ms: records
            .iter()
            .map(|record| record.updated_at_ms)
            .max()
            .unwrap_or(0),
        completed_at_ms: (completed_request_count == request_count)
            .then(|| {
                records
                    .iter()
                    .filter_map(|record| record.completed_at_ms)
                    .max()
            })
            .flatten(),
        request_count,
        completed_request_count,
        expected_sample_count,
    }
}

fn aggregate_generation_batch_status(records: &[RunHistoryRecord]) -> GenerationBatchHistoryStatus {
    for (status, aggregate) in [
        (
            RunHistoryStatus::Paused,
            GenerationBatchHistoryStatus::Paused,
        ),
        (
            RunHistoryStatus::Running,
            GenerationBatchHistoryStatus::Running,
        ),
        (
            RunHistoryStatus::Preparing,
            GenerationBatchHistoryStatus::Preparing,
        ),
        (
            RunHistoryStatus::Waiting,
            GenerationBatchHistoryStatus::Waiting,
        ),
        (
            RunHistoryStatus::Queued,
            GenerationBatchHistoryStatus::Queued,
        ),
    ] {
        if records.iter().any(|record| record.status == status) {
            return aggregate;
        }
    }
    let succeeded = records
        .iter()
        .filter(|record| record.status == RunHistoryStatus::Succeeded)
        .count();
    if !records.is_empty() && succeeded == records.len() {
        return GenerationBatchHistoryStatus::Succeeded;
    }
    if succeeded > 0 {
        return GenerationBatchHistoryStatus::PartiallySucceeded;
    }
    if records
        .iter()
        .any(|record| record.status == RunHistoryStatus::Failed)
    {
        return GenerationBatchHistoryStatus::Failed;
    }
    GenerationBatchHistoryStatus::Stopped
}

fn history_error(error: impl std::fmt::Display) -> AppError {
    AppError::new("run_history", error.to_string())
}

pub async fn ensure_generation_history_target_is_new(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job_id: &str,
) -> AppResult<()> {
    if repository
        .get_run_history(job_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?
        .is_some()
    {
        return Err(AppError::new(
            "invalid_request",
            "generation job_id already exists in run history",
        ));
    }
    if repository
        .run_history_batch_exists(batch_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?
    {
        return Err(AppError::new(
            "invalid_request",
            "generation batch_id already exists in run history",
        ));
    }
    Ok(())
}

pub async fn upsert_generation_history_record(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job_id: &str,
    update: GenerationHistoryUpdate,
) -> AppResult<RunHistoryRecord> {
    let now = unix_timestamp_ms();
    let existing = repository
        .get_run_history(job_id)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    let created_at_ms = existing.as_ref().map_or(now, |record| record.created_at_ms);
    let record = RunHistoryRecord {
        run_id: job_id.to_owned(),
        kind: RunHistoryKind::Generation,
        status: update.status,
        batch_id: Some(batch_id.to_owned()),
        job_id: Some(job_id.to_owned()),
        origin_run_id: update.origin_run_id.or_else(|| {
            existing
                .as_ref()
                .and_then(|record| record.origin_run_id.clone())
        }),
        request_index: update
            .position
            .map(|value| value.request_index)
            .or_else(|| existing.as_ref().and_then(|record| record.request_index)),
        expected_samples: update
            .position
            .map(|value| value.expected_samples.max(1))
            .or_else(|| existing.as_ref().and_then(|record| record.expected_samples)),
        submitted_payload_ref: Some(format!("generation-submitted:{job_id}")),
        prepared_payload_ref: existing
            .as_ref()
            .and_then(|record| record.prepared_payload_ref.clone()),
        title: update
            .title
            .or_else(|| existing.as_ref().and_then(|record| record.title.clone())),
        last_error: update.last_error,
        created_at_ms,
        updated_at_ms: now,
        completed_at_ms: status_is_terminal(update.status).then_some(now),
        recoverable: update.status == RunHistoryStatus::Paused,
    };
    repository
        .upsert_run_history(record.clone())
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    Ok(record)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GenerationHistoryPosition {
    pub request_index: u32,
    pub expected_samples: u32,
}

pub struct GenerationHistoryUpdate {
    pub status: RunHistoryStatus,
    pub title: Option<String>,
    pub origin_run_id: Option<String>,
    pub last_error: Option<String>,
    pub position: Option<GenerationHistoryPosition>,
}

pub async fn generation_history_records_from_queue_snapshot(
    repository: &DatabaseRunHistoryRepository,
    snapshot: &JobQueueSnapshot,
) -> AppResult<Vec<RunHistoryRecord>> {
    let Some(active_batch) = &snapshot.active_batch else {
        return Ok(Vec::new());
    };
    let paused_job_id = if active_batch.batch.status == BatchStatus::Paused {
        active_batch.current_job.as_ref().or_else(|| {
            active_batch
                .batch
                .jobs
                .iter()
                .find(|job| !job.status.is_terminal())
                .map(|job| &job.job_id)
        })
    } else {
        None
    };
    let mut records = Vec::new();
    for (request_index, job) in active_batch.batch.jobs.iter().enumerate() {
        if job.kind != JobKind::GenerateImage {
            continue;
        }
        let status = if paused_job_id.is_some_and(|job_id| job_id == &job.job_id) {
            RunHistoryStatus::Paused
        } else {
            run_history_status_from_job_status(job.status)
        };
        records.push(
            build_generation_history_from_job(
                repository,
                active_batch.batch.batch_id.as_str(),
                job,
                status,
                u32::try_from(request_index).unwrap_or(u32::MAX),
            )
            .await?,
        );
    }
    Ok(records)
}

async fn build_generation_history_from_job(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job: &JobRecord,
    status: RunHistoryStatus,
    request_index: u32,
) -> AppResult<RunHistoryRecord> {
    let now = unix_timestamp_ms();
    let existing = repository
        .get_run_history(job.job_id.as_str())
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    let record = RunHistoryRecord {
        run_id: job.job_id.as_str().to_owned(),
        kind: RunHistoryKind::Generation,
        status,
        batch_id: Some(batch_id.to_owned()),
        job_id: Some(job.job_id.as_str().to_owned()),
        origin_run_id: existing
            .as_ref()
            .and_then(|record| record.origin_run_id.clone()),
        request_index: existing
            .as_ref()
            .and_then(|record| record.request_index)
            .or(Some(request_index)),
        expected_samples: existing
            .as_ref()
            .and_then(|record| record.expected_samples)
            .or(Some(1)),
        submitted_payload_ref: Some(job.payload_ref.as_str().to_owned()),
        prepared_payload_ref: job
            .prepared_payload_ref
            .as_ref()
            .map(|id| id.as_str().to_owned())
            .or_else(|| {
                existing
                    .as_ref()
                    .and_then(|record| record.prepared_payload_ref.clone())
            }),
        title: existing.as_ref().and_then(|record| record.title.clone()),
        last_error: existing
            .as_ref()
            .and_then(|record| record.last_error.clone()),
        created_at_ms: existing.as_ref().map_or(now, |record| record.created_at_ms),
        updated_at_ms: now,
        completed_at_ms: if status_is_terminal(status) {
            existing
                .as_ref()
                .and_then(|record| record.completed_at_ms)
                .or(Some(now))
        } else {
            None
        },
        recoverable: status == RunHistoryStatus::Paused,
    };
    Ok(record)
}

const fn run_history_status_from_job_status(status: JobStatus) -> RunHistoryStatus {
    match status {
        JobStatus::Queued => RunHistoryStatus::Queued,
        JobStatus::Preparing => RunHistoryStatus::Preparing,
        JobStatus::Running => RunHistoryStatus::Running,
        JobStatus::WaitingRetry => RunHistoryStatus::Waiting,
        JobStatus::Blocked => RunHistoryStatus::Paused,
        JobStatus::Succeeded => RunHistoryStatus::Succeeded,
        JobStatus::Failed => RunHistoryStatus::Failed,
        JobStatus::Skipped => RunHistoryStatus::Skipped,
    }
}

const fn status_is_terminal(status: RunHistoryStatus) -> bool {
    matches!(
        status,
        RunHistoryStatus::Succeeded
            | RunHistoryStatus::Failed
            | RunHistoryStatus::Skipped
            | RunHistoryStatus::Stopped
    )
}

fn unix_timestamp_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(millis).unwrap_or(u64::MAX)
}
