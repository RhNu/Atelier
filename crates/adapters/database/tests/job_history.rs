use std::time::Duration;

use futures_executor::block_on;
use nai_atelier_adapter_database::{
    DatabaseConnection, DatabaseJobQueueRepository, DatabaseRunHistoryRepository,
};
use nai_atelier_jobs::{
    BatchId, BatchStatus, JobFailureImpact, JobId, JobKind, JobPayloadRef, JobQueue,
    JobQueueRepository, JobStatus, QueueDelay, RunHistoryKind, RunHistoryQuery, RunHistoryRecord,
    RunHistoryRepository, RunHistoryStatus, RunOutputRecord, SubmitJob,
};
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

fn director_history() -> RunHistoryRecord {
    RunHistoryRecord {
        run_id: "director-1".to_owned(),
        kind: RunHistoryKind::Director,
        status: RunHistoryStatus::Failed,
        batch_id: None,
        job_id: None,
        origin_run_id: None,
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
        artifact_id: "artifact:job-1:sample:0".to_owned(),
        item_id: Some("artifact:artifact:job-1:sample:0".to_owned()),
        resource_id: "resource:job-1:sample:0".to_owned(),
        variant_id: Some("variant:job-1:preview".to_owned()),
        asset_role: "preview".to_owned(),
        variant_kind: Some("preview".to_owned()),
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
