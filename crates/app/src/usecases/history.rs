use nai_atelier_adapter_database::DatabaseRunHistoryRepository;
use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::generation::QueueDirectiveDto;
use nai_atelier_app_api::history::{
    RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto, RunHistoryPageDto,
    RunHistoryQueryDto,
};
use nai_atelier_jobs::{
    BatchId, BatchStatus, JobId, JobKind, JobPayloadRef, JobQueueRepository, JobQueueSnapshot,
    JobRecord, JobStatus, RunHistoryKind, RunHistoryRecord, RunHistoryRepository, RunHistoryStatus,
};
use nai_atelier_kernel::{GenerationPayloadStore, SubmitGenerationWork};
use nai_atelier_secrets::{SecretStore, SecretsErrorKind};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::AtelierApp;
use crate::mapping::{
    queue_directive_to_dto, run_history_item_to_dto, run_history_page_to_dto,
    run_history_query_to_domain,
};
use crate::{AppError, AppResult};

pub struct HistoryUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> HistoryUseCases<'_, S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: nai_atelier_vibe::EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
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
        if matches!(directive, QueueDirectiveDto::Idle) {
            self.app
                .inner
                .queue_repository
                .clear_queue_snapshot()
                .await
                .map_err(|error| AppError::new("job_queue", error.to_string()))?;
        } else {
            self.app
                .inner
                .queue_repository
                .save_queue_snapshot(&snapshot)
                .await
                .map_err(|error| AppError::new("job_queue", error.to_string()))?;
        }
        let record = upsert_generation_history_record(
            &self.app.inner.run_history,
            &request.batch_id,
            &request.job_id,
            RunHistoryStatus::Queued,
            Some(title),
            Some(request.run_id),
            None,
        )
        .await?;
        Ok(RerunGenerationHistoryItemResponseDto {
            directive,
            item: run_history_item_to_dto(record, Vec::new()),
        })
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
    status: RunHistoryStatus,
    title: Option<String>,
    origin_run_id: Option<String>,
    last_error: Option<String>,
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
        status,
        batch_id: Some(batch_id.to_owned()),
        job_id: Some(job_id.to_owned()),
        origin_run_id: origin_run_id.or_else(|| {
            existing
                .as_ref()
                .and_then(|record| record.origin_run_id.clone())
        }),
        submitted_payload_ref: Some(format!("generation-submitted:{job_id}")),
        prepared_payload_ref: existing
            .as_ref()
            .and_then(|record| record.prepared_payload_ref.clone()),
        title: title.or_else(|| existing.as_ref().and_then(|record| record.title.clone())),
        last_error,
        created_at_ms,
        updated_at_ms: now,
        completed_at_ms: status_is_terminal(status).then_some(now),
        recoverable: status == RunHistoryStatus::Paused,
    };
    repository
        .upsert_run_history(record.clone())
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))?;
    Ok(record)
}

pub async fn sync_generation_history_from_queue_snapshot(
    repository: &DatabaseRunHistoryRepository,
    snapshot: &JobQueueSnapshot,
) -> AppResult<()> {
    let Some(active_batch) = &snapshot.active_batch else {
        return Ok(());
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
    for job in &active_batch.batch.jobs {
        if job.kind != JobKind::GenerateImage {
            continue;
        }
        let status = if paused_job_id.is_some_and(|job_id| job_id == &job.job_id) {
            RunHistoryStatus::Paused
        } else {
            run_history_status_from_job_status(job.status)
        };
        upsert_generation_history_from_job(
            repository,
            active_batch.batch.batch_id.as_str(),
            job,
            status,
        )
        .await?;
    }
    Ok(())
}

async fn upsert_generation_history_from_job(
    repository: &DatabaseRunHistoryRepository,
    batch_id: &str,
    job: &JobRecord,
    status: RunHistoryStatus,
) -> AppResult<()> {
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
    repository
        .upsert_run_history(record)
        .await
        .map_err(|error| AppError::new("run_history", error.to_string()))
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
