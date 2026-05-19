use std::time::Duration;

use nai_atelier_foundation::{NovelAiError, NovelAiErrorKind};
use nai_atelier_jobs::{
    BatchId, BatchStatus, JobFailureImpact, JobId, JobKind, JobPayloadRef, JobQueue,
    JobQueueErrorKind, JobStatus, QueueDelay, QueueDirective, RetryPolicy, SubmitJob,
};

#[test]
fn submit_batch_starts_first_job_and_rejects_any_active_batch() {
    let mut queue = JobQueue::default();
    let first = job("job-1");
    let second = job("job-2");

    let directive = queue
        .submit_batch(BatchId::new("batch-1"), vec![first.clone(), second])
        .unwrap();

    assert_eq!(directive, QueueDirective::StartJob(first.job_id.clone()));
    assert_eq!(queue.batch_status(), Some(BatchStatus::Running));
    assert_eq!(queue.job_status(&first.job_id), Some(JobStatus::Queued));

    let conflict = queue
        .submit_batch(BatchId::new("batch-2"), vec![job("job-3")])
        .unwrap_err();

    assert_eq!(conflict.kind(), JobQueueErrorKind::Conflict);
}

#[test]
fn submit_batch_rejects_duplicate_job_ids() {
    let mut queue = JobQueue::default();
    let error = queue
        .submit_batch(BatchId::new("batch"), vec![job("job-1"), job("job-1")])
        .unwrap_err();

    assert_eq!(error.kind(), JobQueueErrorKind::Conflict);
}

#[test]
fn successful_jobs_wait_three_seconds_between_items_but_not_before_first() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");
    let second = JobId::new("job-2");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    let directive = queue.mark_succeeded(&first).unwrap();

    assert_eq!(
        directive,
        QueueDirective::Wait(QueueDelay::fixed(Duration::from_secs(3)))
    );
    assert_eq!(queue.batch_status(), Some(BatchStatus::Waiting));

    let directive = queue.delay_elapsed().unwrap();

    assert_eq!(directive, QueueDirective::StartJob(second));
    assert_eq!(queue.batch_status(), Some(BatchStatus::Running));
}

#[test]
fn rate_limited_jobs_retry_three_times_with_specific_delay_only() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();

    let retry_after = Duration::from_secs(24);
    let directive = queue
        .mark_failed(
            &first,
            JobFailureImpact::from_novelai_error(
                &NovelAiError::new(NovelAiErrorKind::RateLimited, "slow down")
                    .with_status(429)
                    .with_retry_after(retry_after),
            ),
        )
        .unwrap();

    assert_eq!(
        directive,
        QueueDirective::Wait(QueueDelay::fixed(retry_after))
    );
    assert_eq!(queue.job_status(&first), Some(JobStatus::WaitingRetry));
    assert_eq!(queue.retry_attempts(&first), Some(1));

    let directive = queue.delay_elapsed().unwrap();

    assert_eq!(directive, QueueDirective::StartJob(first.clone()));
    assert_eq!(
        queue.prepared_payload_ref(&first),
        Some(&JobPayloadRef::new("prepared:job-1"))
    );

    for expected_attempts in 2..=3 {
        queue
            .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
            .unwrap();
        let directive = queue
            .mark_failed(
                &first,
                JobFailureImpact::from_novelai_error(&NovelAiError::new(
                    NovelAiErrorKind::RateLimited,
                    "slow down",
                )),
            )
            .unwrap();
        assert_eq!(
            directive,
            QueueDirective::Wait(QueueDelay::range(
                Duration::from_secs(20),
                Duration::from_secs(30)
            ))
        );
        assert_eq!(queue.retry_attempts(&first), Some(expected_attempts));
        queue.delay_elapsed().unwrap();
    }

    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    let directive = queue
        .mark_failed(
            &first,
            JobFailureImpact::from_novelai_error(&NovelAiError::new(
                NovelAiErrorKind::RateLimited,
                "slow down",
            )),
        )
        .unwrap();

    assert_eq!(directive, QueueDirective::Paused);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Paused));
    assert_eq!(queue.job_status(&first), Some(JobStatus::Blocked));
    assert_eq!(queue.retry_attempts(&first), Some(3));
}

#[test]
fn invalid_request_fails_current_job_and_continues_to_next() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");
    let second = JobId::new("job-2");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    let directive = queue
        .mark_failed(
            &first,
            JobFailureImpact::from_novelai_error(&NovelAiError::new(
                NovelAiErrorKind::InvalidRequest,
                "bad params",
            )),
        )
        .unwrap();

    assert_eq!(queue.job_status(&first), Some(JobStatus::Failed));
    assert_eq!(
        directive,
        QueueDirective::Wait(QueueDelay::fixed(Duration::from_secs(3)))
    );

    assert_eq!(
        queue.delay_elapsed().unwrap(),
        QueueDirective::StartJob(second)
    );
}

#[test]
fn global_errors_pause_and_resume_retries_the_same_prepared_job() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    let directive = queue
        .mark_failed(
            &first,
            JobFailureImpact::from_novelai_error(&NovelAiError::new(
                NovelAiErrorKind::UnknownApi,
                "changed upstream",
            )),
        )
        .unwrap();

    assert_eq!(directive, QueueDirective::Paused);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Paused));
    assert_eq!(queue.job_status(&first), Some(JobStatus::Blocked));
    assert_eq!(queue.retry_attempts(&first), Some(0));

    let directive = queue.resume().unwrap();

    assert_eq!(directive, QueueDirective::StartJob(first.clone()));
    assert_eq!(
        queue.prepared_payload_ref(&first),
        Some(&JobPayloadRef::new("prepared:job-1"))
    );
}

#[test]
fn pause_during_wait_freezes_delay_and_resume_continues_waiting() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    queue.mark_succeeded(&first).unwrap();

    assert_eq!(queue.pause().unwrap(), QueueDirective::Paused);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Paused));
    assert_eq!(
        queue.paused_delay(),
        Some(QueueDelay::fixed(Duration::from_secs(3)))
    );

    assert_eq!(
        queue.resume().unwrap(),
        QueueDirective::Wait(QueueDelay::fixed(Duration::from_secs(3)))
    );
}

#[test]
fn graceful_stop_allows_running_job_to_finish_and_skips_unstarted_jobs() {
    let mut queue = active_queue(["job-1", "job-2", "job-3"]);
    let first = JobId::new("job-1");
    let second = JobId::new("job-2");
    let third = JobId::new("job-3");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();

    assert_eq!(queue.stop().unwrap(), QueueDirective::Paused);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Stopping));

    let directive = queue.mark_succeeded(&first).unwrap();

    assert_eq!(directive, QueueDirective::Idle);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Stopped));
    assert_eq!(queue.job_status(&first), Some(JobStatus::Succeeded));
    assert_eq!(queue.job_status(&second), Some(JobStatus::Skipped));
    assert_eq!(queue.job_status(&third), Some(JobStatus::Skipped));
    assert!(queue.resume().is_err());
}

#[test]
fn delay_and_retry_waits_cannot_be_bypassed_by_starting_other_jobs() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");
    let second = JobId::new("job-2");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    queue.mark_succeeded(&first).unwrap();

    assert_eq!(
        queue.start_next().unwrap_err().kind(),
        JobQueueErrorKind::InvalidState
    );
    assert_eq!(
        queue.mark_preparing(&second).unwrap_err().kind(),
        JobQueueErrorKind::InvalidState
    );
    assert_eq!(
        queue
            .mark_running(&second, JobPayloadRef::new("prepared:job-2"))
            .unwrap_err()
            .kind(),
        JobQueueErrorKind::InvalidState
    );

    queue.delay_elapsed().unwrap();
    queue.mark_preparing(&second).unwrap();
    queue
        .mark_running(&second, JobPayloadRef::new("prepared:job-2"))
        .unwrap();
    queue
        .mark_failed(
            &second,
            JobFailureImpact::from_novelai_error(&NovelAiError::new(
                NovelAiErrorKind::RateLimited,
                "slow down",
            )),
        )
        .unwrap();

    assert_eq!(
        queue.start_next().unwrap_err().kind(),
        JobQueueErrorKind::InvalidState
    );
    assert_eq!(
        queue.mark_preparing(&first).unwrap_err().kind(),
        JobQueueErrorKind::InvalidState
    );
}

#[test]
fn graceful_stop_is_honored_when_current_job_fails() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");
    let second = JobId::new("job-2");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    queue.stop().unwrap();
    let directive = queue
        .mark_failed(&first, JobFailureImpact::FailCurrentAndContinue)
        .unwrap();

    assert_eq!(directive, QueueDirective::Idle);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Stopped));
    assert_eq!(queue.job_status(&first), Some(JobStatus::Failed));
    assert_eq!(queue.job_status(&second), Some(JobStatus::Skipped));
}

#[test]
fn pause_after_running_job_is_honored_when_current_job_fails() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    queue.pause().unwrap();
    let directive = queue
        .mark_failed(&first, JobFailureImpact::FailCurrentAndContinue)
        .unwrap();

    assert_eq!(directive, QueueDirective::Paused);
    assert_eq!(queue.batch_status(), Some(BatchStatus::Paused));
    assert_eq!(
        queue.paused_delay(),
        Some(QueueDelay::fixed(Duration::from_secs(3)))
    );
}

#[test]
fn pause_flag_is_cleared_when_global_failure_already_pauses_current_job() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    queue.pause().unwrap();
    queue
        .mark_failed(&first, JobFailureImpact::PauseAndRetryCurrent)
        .unwrap();

    assert_eq!(
        queue.resume().unwrap(),
        QueueDirective::StartJob(first.clone())
    );
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();

    assert_eq!(
        queue.mark_succeeded(&first).unwrap(),
        QueueDirective::Wait(QueueDelay::fixed(Duration::from_secs(3)))
    );
}

#[test]
fn failure_transitions_only_apply_to_the_current_running_job() {
    let mut queue = active_queue(["job-1", "job-2"]);
    let first = JobId::new("job-1");
    let second = JobId::new("job-2");

    assert_eq!(
        queue
            .mark_failed(&second, JobFailureImpact::FailCurrentAndContinue)
            .unwrap_err()
            .kind(),
        JobQueueErrorKind::InvalidState
    );

    queue.mark_preparing(&first).unwrap();
    queue
        .mark_running(&first, JobPayloadRef::new("prepared:job-1"))
        .unwrap();
    assert_eq!(
        queue
            .mark_failed(&second, JobFailureImpact::PauseAndRetryCurrent)
            .unwrap_err()
            .kind(),
        JobQueueErrorKind::InvalidState
    );

    queue.mark_succeeded(&first).unwrap();
    assert_eq!(
        queue
            .mark_failed(
                &first,
                JobFailureImpact::RetryAfter(QueueDelay::fixed(Duration::from_secs(20)))
            )
            .unwrap_err()
            .kind(),
        JobQueueErrorKind::InvalidState
    );
    assert_eq!(queue.job_status(&first), Some(JobStatus::Succeeded));
}

#[test]
fn custom_retry_policy_classifies_rate_limit_with_custom_fallback_delay() {
    let policy = RetryPolicy {
        rate_limit_fallback: QueueDelay::fixed(Duration::from_secs(42)),
        ..RetryPolicy::default()
    };
    let impact = JobFailureImpact::from_novelai_error_with_policy(
        &NovelAiError::new(NovelAiErrorKind::RateLimited, "slow down"),
        policy,
    );

    assert_eq!(
        impact,
        JobFailureImpact::RetryAfter(QueueDelay::fixed(Duration::from_secs(42)))
    );
}

fn active_queue<const N: usize>(ids: [&str; N]) -> JobQueue {
    let mut queue = JobQueue::default();
    let jobs = ids.into_iter().map(job).collect();
    queue.submit_batch(BatchId::new("batch"), jobs).unwrap();
    queue
}

fn job(id: &str) -> SubmitJob {
    SubmitJob {
        job_id: JobId::new(id),
        kind: JobKind::GenerateImage,
        payload_ref: JobPayloadRef::new(format!("payload:{id}")),
    }
}
