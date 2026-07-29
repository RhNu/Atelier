use std::time::Duration;

use atelier_adapter_database::{
    DatabaseConnection, DatabaseJobQueueRepository, DatabaseRunHistoryRepository,
};
use atelier_jobs::{
    BatchId, BatchStatus, GenerationBatchHistoryQuery, GenerationBatchHistoryStatus,
    JobFailureImpact, JobId, JobKind, JobPayloadRef, JobQueue, JobQueueRepository, JobStatus,
    QueueDelay, RunHistoryKind, RunHistoryQuery, RunHistoryRecord, RunHistoryRepository,
    RunHistoryStatus, RunOutputRecord, RunOutputState, SubmitJob,
};
use futures_executor::block_on;
use rusqlite::Connection;

#[test]
fn job_queue_repository_round_trips_active_snapshot() {
    block_on(async {
        let repository =
            DatabaseJobQueueRepository::new(DatabaseConnection::open_memory().unwrap());
        let mut queue = JobQueue::default();
        queue
            .submit_batch(BatchId::new("batch-1"), vec![job("job-1"), job("job-2")])
            .unwrap();
        let first = JobId::new("job-1");
        queue.mark_preparing(&first).unwrap();
        queue
            .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
            .unwrap();
        queue
            .mark_failed(
                &first,
                JobFailureImpact::RetryAfter(QueueDelay::fixed(Duration::from_secs(17))),
            )
            .unwrap();

        repository
            .save_queue_snapshot(&queue.snapshot())
            .await
            .unwrap();
        let restored_snapshot = repository.load_queue_snapshot().await.unwrap().unwrap();
        let restored = JobQueue::from_snapshot(restored_snapshot).unwrap();

        assert_eq!(restored.batch_status(), Some(BatchStatus::Waiting));
        assert_eq!(restored.job_status(&first), Some(JobStatus::WaitingRetry));
        assert_eq!(restored.retry_attempts(&first), Some(1));

        repository.clear_queue_snapshot().await.unwrap();
        assert!(repository.load_queue_snapshot().await.unwrap().is_none());
    });
}

#[test]
fn run_history_repository_round_trips_records_outputs_and_queries() {
    block_on(async {
        let repository =
            DatabaseRunHistoryRepository::new(DatabaseConnection::open_memory().unwrap());
        let generation = generation_history();
        let director = director_history();
        let output = preview_output();

        repository
            .upsert_run_history(generation.clone())
            .await
            .unwrap();
        repository
            .upsert_run_history(director.clone())
            .await
            .unwrap();
        repository.upsert_run_output(output.clone()).await.unwrap();

        assert_eq!(
            repository.get_run_history("job-1").await.unwrap().unwrap(),
            generation
        );
        assert_eq!(
            repository
                .query_run_history(RunHistoryQuery {
                    kind: Some(RunHistoryKind::Generation),
                    offset: 0,
                    limit: 10,
                    ..RunHistoryQuery::default()
                })
                .await
                .unwrap(),
            vec![generation]
        );
        assert_eq!(
            repository.list_run_outputs("job-1").await.unwrap(),
            vec![output]
        );
    });
}

#[test]
fn run_history_repository_counts_and_checks_batch_ids() {
    block_on(async {
        let repository =
            DatabaseRunHistoryRepository::new(DatabaseConnection::open_memory().unwrap());
        repository
            .upsert_run_history(generation_history())
            .await
            .unwrap();
        repository
            .upsert_run_history(director_history())
            .await
            .unwrap();

        assert_eq!(
            repository
                .count_run_history(RunHistoryQuery {
                    kind: Some(RunHistoryKind::Generation),
                    ..RunHistoryQuery::default()
                })
                .await
                .unwrap(),
            1
        );
        assert!(
            repository
                .run_history_batch_exists("batch-1")
                .await
                .unwrap()
        );
        assert!(
            !repository
                .run_history_batch_exists("batch-missing")
                .await
                .unwrap()
        );
    });
}

#[test]
fn generation_batch_history_aggregates_pages_orders_and_deletes_requests() {
    block_on(async {
        let repository =
            DatabaseRunHistoryRepository::new(DatabaseConnection::open_memory().unwrap());
        let first = generation_request("job-a0", "batch-a", 0, RunHistoryStatus::Succeeded, 2, 100);
        let second = generation_request("job-a1", "batch-a", 1, RunHistoryStatus::Failed, 1, 200);
        let latest =
            generation_request("job-b0", "batch-b", 0, RunHistoryStatus::Succeeded, 1, 300);
        for record in [&first, &second, &latest] {
            repository.upsert_run_history(record.clone()).await.unwrap();
        }
        let mut sample_one = original_output();
        sample_one.run_id = first.run_id.clone();
        sample_one.sample_index = Some(1);
        let mut sample_zero = sample_one.clone();
        sample_zero.sample_index = Some(0);
        sample_zero.artifact_id = "artifact:job-a0:sample:0".to_owned();
        repository.upsert_run_output(sample_one).await.unwrap();
        repository.upsert_run_output(sample_zero).await.unwrap();

        let batches = repository
            .query_generation_batches(GenerationBatchHistoryQuery::default())
            .await
            .unwrap();
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.batch_id.as_str())
                .collect::<Vec<_>>(),
            vec!["batch-b", "batch-a"]
        );
        assert_eq!(
            batches[1].status,
            GenerationBatchHistoryStatus::PartiallySucceeded
        );
        assert_eq!(batches[1].request_count, 2);
        assert_eq!(batches[1].completed_request_count, 2);
        assert_eq!(batches[1].expected_sample_count, 3);
        assert_eq!(
            repository
                .count_generation_batches(GenerationBatchHistoryQuery {
                    status: Some(GenerationBatchHistoryStatus::PartiallySucceeded),
                    ..GenerationBatchHistoryQuery::default()
                })
                .await
                .unwrap(),
            1
        );

        let requests = repository
            .list_run_history_by_batch("batch-a")
            .await
            .unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|record| record.run_id.as_str())
                .collect::<Vec<_>>(),
            vec!["job-a0", "job-a1"]
        );
        assert_eq!(
            repository
                .list_run_outputs("job-a0")
                .await
                .unwrap()
                .iter()
                .map(|output| output.sample_index)
                .collect::<Vec<_>>(),
            vec![Some(0), Some(1)]
        );

        assert_eq!(
            repository
                .delete_generation_batches(&["batch-a".to_owned()])
                .await
                .unwrap(),
            2
        );
        assert!(
            repository
                .list_run_history_by_batch("batch-a")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            repository
                .list_run_outputs("job-a0")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            repository
                .get_run_history("job-b0")
                .await
                .unwrap()
                .is_some()
        );
    });
}

#[test]
fn run_output_upsert_replaces_records_without_variant_id() {
    block_on(async {
        let repository =
            DatabaseRunHistoryRepository::new(DatabaseConnection::open_memory().unwrap());
        let output = original_output();
        repository
            .upsert_run_history(generation_history())
            .await
            .unwrap();
        repository.upsert_run_output(output.clone()).await.unwrap();
        repository.upsert_run_output(output.clone()).await.unwrap();

        assert_eq!(
            repository.list_run_outputs("job-1").await.unwrap(),
            vec![output]
        );
    });
}

#[test]
fn deleted_run_outputs_clear_stale_gallery_and_resource_references() {
    block_on(async {
        let repository =
            DatabaseRunHistoryRepository::new(DatabaseConnection::open_memory().unwrap());
        repository
            .upsert_run_history(generation_history())
            .await
            .unwrap();
        let mut output = original_output();
        output.item_id = Some("gallery:item-1".to_owned());
        output.variant_id = Some("preview".to_owned());
        repository.upsert_run_output(output).await.unwrap();

        assert_eq!(
            repository
                .mark_run_outputs_deleted_by_item_ids(&["gallery:item-1".to_owned()])
                .await
                .unwrap(),
            1
        );
        let tombstone = repository
            .list_run_outputs("job-1")
            .await
            .unwrap()
            .remove(0);
        assert_eq!(tombstone.state, RunOutputState::Deleted);
        assert_eq!(tombstone.item_id, None);
        assert_eq!(tombstone.resource_id, None);
        assert_eq!(tombstone.variant_id, None);
    });
}

#[test]
fn run_history_delete_removes_records_and_associated_outputs_only() {
    block_on(async {
        let repository =
            DatabaseRunHistoryRepository::new(DatabaseConnection::open_memory().unwrap());
        repository
            .upsert_run_history(generation_history())
            .await
            .unwrap();
        repository
            .upsert_run_history(director_history())
            .await
            .unwrap();
        repository
            .upsert_run_output(original_output())
            .await
            .unwrap();

        let deleted = repository
            .delete_run_history_items(&["job-1".to_owned(), "missing".to_owned()])
            .await
            .unwrap();

        assert_eq!(deleted, 1);
        assert!(repository.get_run_history("job-1").await.unwrap().is_none());
        assert!(
            repository
                .get_run_history("director-1")
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            repository
                .list_run_outputs("job-1")
                .await
                .unwrap()
                .is_empty()
        );
    });
}

#[test]
fn migrations_add_queue_and_history_tables_to_existing_v3_database() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                r"
                CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_ms INTEGER NOT NULL DEFAULT (
                        CAST(strftime('%s', 'now') AS INTEGER) * 1000
                    )
                );
                INSERT INTO schema_migrations(version) VALUES (1);
                INSERT INTO schema_migrations(version) VALUES (2);
                INSERT INTO schema_migrations(version) VALUES (3);
                ",
            )
            .unwrap();
    }

    let connection = DatabaseConnection::open(&path).unwrap();
    let repository = DatabaseRunHistoryRepository::new(connection);
    block_on(async {
        repository
            .upsert_run_history(RunHistoryRecord {
                run_id: "job-1".to_owned(),
                kind: RunHistoryKind::Generation,
                status: RunHistoryStatus::Queued,
                batch_id: Some("batch-1".to_owned()),
                job_id: Some("job-1".to_owned()),
                origin_run_id: None,
                request_index: Some(0),
                expected_samples: Some(1),
                submitted_payload_ref: None,
                prepared_payload_ref: None,
                title: None,
                last_error: None,
                created_at_ms: 1,
                updated_at_ms: 1,
                completed_at_ms: None,
                recoverable: false,
            })
            .await
            .unwrap();
        assert_eq!(
            repository
                .query_run_history(RunHistoryQuery::default())
                .await
                .unwrap()
                .len(),
            1
        );
    });
}

#[test]
fn generation_batch_migration_recovers_legacy_request_and_sample_positions() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                r#"
                CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_ms INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO schema_migrations(version) VALUES (1), (2), (3), (4), (5), (6);

                CREATE TABLE generation_payloads (
                    payload_ref TEXT NOT NULL,
                    payload_kind TEXT NOT NULL,
                    payload_json TEXT NOT NULL,
                    PRIMARY KEY (payload_kind, payload_ref)
                );
                CREATE TABLE gallery_items (
                    item_id TEXT PRIMARY KEY,
                    artifact_id TEXT NOT NULL,
                    artifact_kind TEXT NOT NULL,
                    source_kind TEXT NOT NULL,
                    manual_safety_override TEXT,
                    indexed_at_ms INTEGER NOT NULL,
                    item_json TEXT NOT NULL,
                    effective_safety_label TEXT
                );
                CREATE TABLE run_history (
                    run_id TEXT PRIMARY KEY,
                    run_kind TEXT NOT NULL,
                    run_status TEXT NOT NULL,
                    batch_id TEXT,
                    job_id TEXT,
                    origin_run_id TEXT,
                    submitted_payload_ref TEXT,
                    prepared_payload_ref TEXT,
                    title TEXT,
                    last_error TEXT,
                    created_at_ms INTEGER NOT NULL,
                    updated_at_ms INTEGER NOT NULL,
                    completed_at_ms INTEGER,
                    recoverable INTEGER NOT NULL
                );
                CREATE TABLE run_outputs (
                    run_id TEXT NOT NULL,
                    artifact_id TEXT NOT NULL,
                    item_id TEXT,
                    resource_id TEXT NOT NULL,
                    variant_id TEXT,
                    asset_role TEXT NOT NULL,
                    variant_kind TEXT
                );

                INSERT INTO run_history VALUES
                    ('job-a', 'generation', 'succeeded', 'batch-old', 'job-a', NULL,
                     'payload-a', NULL, NULL, NULL, 10, 10, 10, 0),
                    ('job-b', 'generation', 'succeeded', 'batch-old', 'job-b', NULL,
                     NULL, NULL, NULL, NULL, 10, 10, 10, 0);
                INSERT INTO generation_payloads VALUES (
                    'payload-a',
                    'submitted',
                    '{"request":{"kind":"stream","request":{"base":{"n_samples":3}}}}'
                );
                INSERT INTO gallery_items VALUES (
                    'gallery-b', 'artifact-b', 'generated_image', 'generation', NULL, 10,
                    '{"metadata":{"sample_index":2}}', NULL
                );
                INSERT INTO run_outputs VALUES (
                    'job-b', 'artifact-b', 'gallery-b', 'resource-b', NULL, 'original', 'original'
                );
                "#,
            )
            .unwrap();
    }

    let repository = DatabaseRunHistoryRepository::new(DatabaseConnection::open(&path).unwrap());
    block_on(async {
        let requests = repository
            .list_run_history_by_batch("batch-old")
            .await
            .unwrap();
        assert_eq!(requests[0].request_index, Some(0));
        assert_eq!(requests[1].request_index, Some(1));
        assert_eq!(requests[0].expected_samples, Some(3));
        assert_eq!(requests[1].expected_samples, Some(3));
        assert_eq!(
            repository.list_run_outputs("job-b").await.unwrap()[0].sample_index,
            Some(2)
        );
    });
}

fn job(id: &str) -> SubmitJob {
    SubmitJob {
        job_id: JobId::new(id),
        kind: JobKind::GenerateImage,
        payload_ref: JobPayloadRef::new(format!("payload:{id}")),
    }
}

fn generation_history() -> RunHistoryRecord {
    RunHistoryRecord {
        run_id: "job-1".to_owned(),
        kind: RunHistoryKind::Generation,
        status: RunHistoryStatus::Succeeded,
        batch_id: Some("batch-1".to_owned()),
        job_id: Some("job-1".to_owned()),
        origin_run_id: Some("job-0".to_owned()),
        request_index: Some(0),
        expected_samples: Some(2),
        submitted_payload_ref: Some("generation-submitted:job-1".to_owned()),
        prepared_payload_ref: Some("generation-prepared:job-1".to_owned()),
        title: Some("1girl".to_owned()),
        last_error: None,
        created_at_ms: 100,
        updated_at_ms: 300,
        completed_at_ms: Some(300),
        recoverable: false,
    }
}

fn generation_request(
    run_id: &str,
    batch_id: &str,
    request_index: u32,
    status: RunHistoryStatus,
    expected_samples: u32,
    updated_at_ms: u64,
) -> RunHistoryRecord {
    RunHistoryRecord {
        run_id: run_id.to_owned(),
        job_id: Some(run_id.to_owned()),
        batch_id: Some(batch_id.to_owned()),
        request_index: Some(request_index),
        expected_samples: Some(expected_samples),
        status,
        created_at_ms: updated_at_ms,
        updated_at_ms,
        completed_at_ms: Some(updated_at_ms),
        ..generation_history()
    }
}

fn director_history() -> RunHistoryRecord {
    RunHistoryRecord {
        run_id: "director-1".to_owned(),
        kind: RunHistoryKind::Director,
        status: RunHistoryStatus::Failed,
        batch_id: None,
        job_id: None,
        origin_run_id: None,
        request_index: None,
        expected_samples: None,
        submitted_payload_ref: None,
        prepared_payload_ref: None,
        title: Some("lineart".to_owned()),
        last_error: Some("director failed".to_owned()),
        created_at_ms: 200,
        updated_at_ms: 400,
        completed_at_ms: Some(400),
        recoverable: false,
    }
}

fn preview_output() -> RunOutputRecord {
    RunOutputRecord {
        run_id: "job-1".to_owned(),
        sample_index: Some(0),
        artifact_id: "artifact:job-1:sample:0".to_owned(),
        item_id: Some("artifact:artifact:job-1:sample:0".to_owned()),
        resource_id: Some("resource:job-1:sample:0".to_owned()),
        variant_id: Some("variant:job-1:preview".to_owned()),
        asset_role: "preview".to_owned(),
        variant_kind: Some("preview".to_owned()),
        state: RunOutputState::Available,
    }
}

fn original_output() -> RunOutputRecord {
    RunOutputRecord {
        variant_id: None,
        asset_role: "original".to_owned(),
        variant_kind: Some("original".to_owned()),
        ..preview_output()
    }
}
