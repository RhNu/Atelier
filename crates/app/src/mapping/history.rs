use super::{
    GenerationBatchHistoryQuery, GenerationBatchHistoryRecord, GenerationBatchHistoryStatus,
    GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, ResourceRefDto, RunHistoryItemDto, RunHistoryKind,
    RunHistoryKindDto, RunHistoryOutputDto, RunHistoryPageDto, RunHistoryQuery, RunHistoryQueryDto,
    RunHistoryRecord, RunHistoryStatus, RunHistoryStatusDto, RunOutputRecord,
};

pub const fn run_history_kind_to_dto(value: RunHistoryKind) -> RunHistoryKindDto {
    match value {
        RunHistoryKind::Generation => RunHistoryKindDto::Generation,
        RunHistoryKind::Director => RunHistoryKindDto::Director,
    }
}

pub const fn run_history_kind_to_domain(value: RunHistoryKindDto) -> RunHistoryKind {
    match value {
        RunHistoryKindDto::Generation => RunHistoryKind::Generation,
        RunHistoryKindDto::Director => RunHistoryKind::Director,
    }
}

pub const fn run_history_status_to_dto(value: RunHistoryStatus) -> RunHistoryStatusDto {
    match value {
        RunHistoryStatus::Queued => RunHistoryStatusDto::Queued,
        RunHistoryStatus::Preparing => RunHistoryStatusDto::Preparing,
        RunHistoryStatus::Running => RunHistoryStatusDto::Running,
        RunHistoryStatus::Waiting => RunHistoryStatusDto::Waiting,
        RunHistoryStatus::Paused => RunHistoryStatusDto::Paused,
        RunHistoryStatus::Succeeded => RunHistoryStatusDto::Succeeded,
        RunHistoryStatus::Failed => RunHistoryStatusDto::Failed,
        RunHistoryStatus::Skipped => RunHistoryStatusDto::Skipped,
        RunHistoryStatus::Stopped => RunHistoryStatusDto::Stopped,
    }
}

pub const fn run_history_status_to_domain(value: RunHistoryStatusDto) -> RunHistoryStatus {
    match value {
        RunHistoryStatusDto::Queued => RunHistoryStatus::Queued,
        RunHistoryStatusDto::Preparing => RunHistoryStatus::Preparing,
        RunHistoryStatusDto::Running => RunHistoryStatus::Running,
        RunHistoryStatusDto::Waiting => RunHistoryStatus::Waiting,
        RunHistoryStatusDto::Paused => RunHistoryStatus::Paused,
        RunHistoryStatusDto::Succeeded => RunHistoryStatus::Succeeded,
        RunHistoryStatusDto::Failed => RunHistoryStatus::Failed,
        RunHistoryStatusDto::Skipped => RunHistoryStatus::Skipped,
        RunHistoryStatusDto::Stopped => RunHistoryStatus::Stopped,
    }
}

pub fn run_history_query_to_domain(value: &RunHistoryQueryDto) -> RunHistoryQuery {
    RunHistoryQuery {
        offset: value.offset,
        limit: value.limit,
        kind: value.kind.map(run_history_kind_to_domain),
        status: value.status.map(run_history_status_to_domain),
    }
}

pub const fn generation_batch_history_status_to_dto(
    value: GenerationBatchHistoryStatus,
) -> GenerationBatchHistoryStatusDto {
    match value {
        GenerationBatchHistoryStatus::Queued => GenerationBatchHistoryStatusDto::Queued,
        GenerationBatchHistoryStatus::Preparing => GenerationBatchHistoryStatusDto::Preparing,
        GenerationBatchHistoryStatus::Running => GenerationBatchHistoryStatusDto::Running,
        GenerationBatchHistoryStatus::Waiting => GenerationBatchHistoryStatusDto::Waiting,
        GenerationBatchHistoryStatus::Paused => GenerationBatchHistoryStatusDto::Paused,
        GenerationBatchHistoryStatus::Succeeded => GenerationBatchHistoryStatusDto::Succeeded,
        GenerationBatchHistoryStatus::PartiallySucceeded => {
            GenerationBatchHistoryStatusDto::PartiallySucceeded
        }
        GenerationBatchHistoryStatus::Failed => GenerationBatchHistoryStatusDto::Failed,
        GenerationBatchHistoryStatus::Stopped => GenerationBatchHistoryStatusDto::Stopped,
    }
}

pub const fn generation_batch_history_status_to_domain(
    value: GenerationBatchHistoryStatusDto,
) -> GenerationBatchHistoryStatus {
    match value {
        GenerationBatchHistoryStatusDto::Queued => GenerationBatchHistoryStatus::Queued,
        GenerationBatchHistoryStatusDto::Preparing => GenerationBatchHistoryStatus::Preparing,
        GenerationBatchHistoryStatusDto::Running => GenerationBatchHistoryStatus::Running,
        GenerationBatchHistoryStatusDto::Waiting => GenerationBatchHistoryStatus::Waiting,
        GenerationBatchHistoryStatusDto::Paused => GenerationBatchHistoryStatus::Paused,
        GenerationBatchHistoryStatusDto::Succeeded => GenerationBatchHistoryStatus::Succeeded,
        GenerationBatchHistoryStatusDto::PartiallySucceeded => {
            GenerationBatchHistoryStatus::PartiallySucceeded
        }
        GenerationBatchHistoryStatusDto::Failed => GenerationBatchHistoryStatus::Failed,
        GenerationBatchHistoryStatusDto::Stopped => GenerationBatchHistoryStatus::Stopped,
    }
}

pub fn generation_history_query_to_domain(
    value: &GenerationHistoryQueryDto,
) -> GenerationBatchHistoryQuery {
    GenerationBatchHistoryQuery {
        offset: value.offset,
        limit: value.limit,
        status: value.status.map(generation_batch_history_status_to_domain),
    }
}

pub fn generation_history_batch_to_dto(
    record: GenerationBatchHistoryRecord,
    completed_sample_count: usize,
    outputs: Vec<RunHistoryOutputDto>,
) -> GenerationHistoryBatchDto {
    GenerationHistoryBatchDto {
        batch_id: record.batch_id,
        status: generation_batch_history_status_to_dto(record.status),
        title: record.title,
        last_error: record.last_error,
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
        completed_at_ms: record.completed_at_ms,
        request_count: record.request_count,
        completed_request_count: record.completed_request_count,
        expected_sample_count: record.expected_sample_count,
        completed_sample_count,
        outputs,
    }
}

pub const fn generation_history_page_to_dto(
    items: Vec<GenerationHistoryBatchDto>,
    offset: usize,
    limit: usize,
    total: usize,
) -> GenerationHistoryPageDto {
    GenerationHistoryPageDto {
        items,
        total,
        offset,
        limit,
    }
}

pub fn run_history_item_to_dto(
    record: RunHistoryRecord,
    outputs: Vec<RunOutputRecord>,
) -> RunHistoryItemDto {
    RunHistoryItemDto {
        run_id: record.run_id,
        kind: run_history_kind_to_dto(record.kind),
        status: run_history_status_to_dto(record.status),
        batch_id: record.batch_id,
        job_id: record.job_id,
        origin_run_id: record.origin_run_id,
        title: record.title,
        last_error: record.last_error,
        created_at_ms: record.created_at_ms,
        updated_at_ms: record.updated_at_ms,
        completed_at_ms: record.completed_at_ms,
        recoverable: record.recoverable,
        outputs: outputs.into_iter().map(run_output_to_dto).collect(),
    }
}

pub const fn run_history_page_to_dto(
    items: Vec<RunHistoryItemDto>,
    offset: usize,
    limit: usize,
    total: usize,
) -> RunHistoryPageDto {
    RunHistoryPageDto {
        items,
        total,
        offset,
        limit,
    }
}

pub fn run_output_to_dto(value: RunOutputRecord) -> RunHistoryOutputDto {
    RunHistoryOutputDto {
        sample_index: value.sample_index,
        artifact_id: value.artifact_id,
        item_id: value.item_id,
        resource: ResourceRefDto {
            id: value.resource_id,
            variant_id: value.variant_id,
        },
        asset_role: value.asset_role,
        variant_kind: value.variant_kind,
    }
}
