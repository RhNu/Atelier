use super::{
    GenerationNotificationKind, NativeAtelierRuntime, NotificationLanguage, TauriNotifier,
    current_notification_language, generation_notification,
};
use crate::desktop_system::DesktopSystem;
use atelier_app::GenerationWorkerCancel;
use atelier_app_api::generation::QueueDirectiveDto;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;
#[derive(Clone, Default)]
pub struct DesktopGenerationWorker {
    inner: Arc<Mutex<WorkerState>>,
}

#[derive(Default)]
struct WorkerState {
    current: Option<WorkerRun>,
    pending: Option<QueueDirectiveDto>,
    next_id: u64,
}

struct WorkerRun {
    id: u64,
    cancel: GenerationWorkerCancel,
    handle: Option<tauri::async_runtime::JoinHandle<()>>,
}

struct WorkerStart {
    id: u64,
    directive: QueueDirectiveDto,
    cancel: GenerationWorkerCancel,
}

impl WorkerState {
    const fn is_busy(&self) -> bool {
        self.current.is_some() || self.pending.is_some()
    }

    fn start_or_defer(&mut self, directive: QueueDirectiveDto) -> Option<WorkerStart> {
        if self.current.is_some() {
            self.pending = Some(directive);
            return None;
        }

        let run_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let cancel = GenerationWorkerCancel::new();
        self.current = Some(WorkerRun {
            id: run_id,
            cancel: cancel.clone(),
            handle: None,
        });
        Some(WorkerStart {
            id: run_id,
            directive,
            cancel,
        })
    }

    fn cancel_current(&self) {
        if let Some(run) = &self.current {
            run.cancel.cancel();
        }
    }

    fn cancel_current_and_clear_pending(&mut self) {
        self.cancel_current();
        self.pending = None;
    }

    fn take_current_for_abort(&mut self) -> Option<tauri::async_runtime::JoinHandle<()>> {
        self.pending = None;
        let mut run = self.current.take()?;
        run.cancel.cancel_active();
        run.handle.take()
    }

    fn finish(&mut self, run_id: u64) -> Option<QueueDirectiveDto> {
        if self.current.as_ref().is_some_and(|run| run.id == run_id) {
            self.current = None;
            return self.pending.take();
        }
        None
    }

    fn attach_handle(&mut self, run_id: u64, handle: tauri::async_runtime::JoinHandle<()>) {
        if let Some(run) = &mut self.current
            && run.id == run_id
        {
            run.handle = Some(handle);
        }
    }
}

impl DesktopGenerationWorker {
    pub fn is_busy(&self) -> bool {
        self.inner.lock().map_or(true, |state| state.is_busy())
    }

    pub(super) fn kick(
        &self,
        app_handle: AppHandle,
        host: Arc<NativeAtelierRuntime>,
        system: Arc<DesktopSystem>,
        notification_language: Arc<Mutex<NotificationLanguage>>,
        directive: QueueDirectiveDto,
    ) {
        if !matches!(
            directive,
            QueueDirectiveDto::StartJob { .. } | QueueDirectiveDto::Wait { .. }
        ) {
            return;
        }

        let Some(start) = ({
            let Ok(mut state) = self.inner.lock() else {
                log::warn!("generation worker state is unavailable");
                return;
            };
            state.start_or_defer(directive)
        }) else {
            return;
        };

        self.spawn_run(app_handle, host, system, notification_language, start);
    }

    fn spawn_run(
        &self,
        app_handle: AppHandle,
        host: Arc<NativeAtelierRuntime>,
        system: Arc<DesktopSystem>,
        notification_language: Arc<Mutex<NotificationLanguage>>,
        start: WorkerStart,
    ) {
        let run_id = start.id;
        let worker = self.clone();
        let handle = tauri::async_runtime::spawn(async move {
            let result = host
                .drive_generation_queue(start.directive, start.cancel)
                .await;
            if let Err(error) = result {
                let notification = generation_notification(
                    current_notification_language(&notification_language),
                    GenerationNotificationKind::Failed,
                );
                let notifier = TauriNotifier::new(app_handle.clone());
                let _ = system.notify(notification.title, notification.body, &notifier);
                log::warn!("generation worker stopped with error: {}", error.message);
            }
            if let Some(next_directive) = worker.finish(run_id) {
                worker.kick(
                    app_handle,
                    host,
                    system,
                    notification_language,
                    next_directive,
                );
            }
        });
        self.attach_handle(run_id, handle);
    }

    pub(super) fn cancel(&self) {
        let Ok(state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return;
        };
        state.cancel_current();
    }

    pub(super) fn cancel_and_clear_pending(&self) {
        let Ok(mut state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return;
        };
        state.cancel_current_and_clear_pending();
    }

    pub(super) async fn abort_and_wait(&self) {
        let handle = {
            let Ok(mut state) = self.inner.lock() else {
                log::warn!("generation worker state is unavailable");
                return;
            };
            state.take_current_for_abort()
        };
        if let Some(handle) = handle {
            match futures::future::select(
                handle,
                futures_timer::Delay::new(Duration::from_millis(500)),
            )
            .await
            {
                futures::future::Either::Left((_result, _delay)) => {}
                futures::future::Either::Right((_elapsed, handle)) => {
                    handle.abort();
                    let _ = handle.await;
                }
            }
        }
    }

    fn finish(&self, run_id: u64) -> Option<QueueDirectiveDto> {
        let Ok(mut state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return None;
        };
        state.finish(run_id)
    }

    fn attach_handle(&self, run_id: u64, handle: tauri::async_runtime::JoinHandle<()>) {
        let Ok(mut state) = self.inner.lock() else {
            log::warn!("generation worker state is unavailable");
            return;
        };
        state.attach_handle(run_id, handle);
    }
}

#[cfg(test)]
mod tests {
    use atelier_app_api::generation::{QueueDelayDto, QueueDirectiveDto};

    use super::{
        GenerationNotificationKind, NotificationLanguage, WorkerState, generation_notification,
    };

    #[test]
    fn generation_notifications_are_localized_and_do_not_include_job_ids() {
        let english = generation_notification(
            NotificationLanguage::English,
            GenerationNotificationKind::Completed,
        );
        assert_eq!(english.title, "Atelier generation complete");
        assert_eq!(english.body, "Your generated image is ready.");
        assert!(!english.body.contains("job"));

        let chinese = generation_notification(
            NotificationLanguage::SimplifiedChinese,
            GenerationNotificationKind::Failed,
        );
        assert_eq!(chinese.title, "Atelier 生成失败");
        assert_eq!(chinese.body, "请打开“生成”查看详情。");
        assert!(!chinese.body.contains("job"));
    }

    #[test]
    fn worker_state_defers_cancelled_run_replacement_until_finish() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::Wait {
                delay: QueueDelayDto {
                    min_ms: 10,
                    max_ms: 10,
                },
            })
            .unwrap();

        state.cancel_current();
        let deferred = QueueDirectiveDto::StartJob {
            job_id: "job-2".to_owned(),
        };

        assert!(state.start_or_defer(deferred.clone()).is_none());
        assert_eq!(state.finish(first.id), Some(deferred.clone()));

        let second = state.start_or_defer(deferred).unwrap();
        assert_ne!(second.id, first.id);
    }

    #[test]
    fn worker_state_retains_next_batch_while_previous_run_finishes() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned(),
            })
            .unwrap();

        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );
        assert_eq!(
            state.finish(first.id),
            Some(QueueDirectiveDto::StartJob {
                job_id: "job-2".to_owned()
            })
        );
    }

    #[test]
    fn worker_state_ignores_stale_finish_after_newer_run_started() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned(),
            })
            .unwrap();
        state.cancel_current();
        let deferred = QueueDirectiveDto::StartJob {
            job_id: "job-2".to_owned(),
        };
        assert!(state.start_or_defer(deferred.clone()).is_none());
        assert_eq!(state.finish(first.id), Some(deferred.clone()));
        let second = state.start_or_defer(deferred).unwrap();

        assert_eq!(state.finish(first.id), None);
        assert!(state.finish(second.id).is_none());
    }

    #[test]
    fn worker_state_clears_pending_for_terminal_cancel() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::Wait {
                delay: QueueDelayDto {
                    min_ms: 10,
                    max_ms: 10,
                },
            })
            .unwrap();
        state.cancel_current();
        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );

        state.cancel_current_and_clear_pending();

        assert_eq!(state.finish(first.id), None);
    }

    #[test]
    fn worker_state_abort_releases_current_run_and_clears_pending() {
        let mut state = WorkerState::default();
        let first = state
            .start_or_defer(QueueDirectiveDto::Wait {
                delay: QueueDelayDto {
                    min_ms: 10,
                    max_ms: 10,
                },
            })
            .unwrap();
        state.cancel_current();
        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );

        assert!(state.take_current_for_abort().is_none());
        assert!(state.current.is_none());
        assert!(state.pending.is_none());
        assert!(first.cancel.is_cancelled());
    }

    #[test]
    fn worker_state_reports_active_and_queued_work_as_busy_for_updater_guard() {
        let mut state = WorkerState::default();
        assert!(!state.is_busy());
        let current = state
            .start_or_defer(QueueDirectiveDto::StartJob {
                job_id: "job-1".to_owned(),
            })
            .unwrap();
        assert!(state.is_busy());
        state.cancel_current();
        assert!(
            state
                .start_or_defer(QueueDirectiveDto::StartJob {
                    job_id: "job-2".to_owned(),
                })
                .is_none()
        );
        assert!(state.is_busy());
        assert!(state.finish(current.id).is_some());
        assert!(!state.is_busy());
    }
}
