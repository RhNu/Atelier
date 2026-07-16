use std::collections::BTreeSet;

use atelier_app_api::history::GenerationHistoryRequestDto;
use atelier_jobs::{
    GenerationBatchHistoryRecord, GenerationBatchHistoryStatus, RunHistoryRecord, RunHistoryStatus,
    RunOutputRecord,
};

use crate::mapping::{run_history_status_to_dto, run_output_to_dto};

use super::persistence::status_is_terminal;

pub(super) fn generation_history_request_to_dto(
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

pub(super) fn preferred_run_outputs(outputs: Vec<RunOutputRecord>) -> Vec<RunOutputRecord> {
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

pub(super) fn aggregate_generation_batch(
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
