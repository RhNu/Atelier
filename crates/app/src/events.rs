use std::sync::{Arc, Mutex};

use nai_atelier_app_api::event::{AppEventDto, AppEventKindDto};
use nai_atelier_generation::GenerationOutputMode;
use nai_atelier_kernel::{KernelEvent, KernelEventKind};

use crate::mapping::resource_ref_to_dto;

const MAX_RETAINED_EVENTS: usize = 1024;

#[derive(Clone, Debug, Default)]
pub struct AppEventHub {
    events: Arc<Mutex<Vec<AppEventDto>>>,
}

impl AppEventHub {
    pub fn push_kernel_event(&self, event: KernelEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(kernel_event_to_dto(event));
            let overflow = events.len().saturating_sub(MAX_RETAINED_EVENTS);
            if overflow > 0 {
                events.drain(..overflow);
            }
        }
    }

    #[must_use]
    pub fn events_since(&self, sequence: u64, limit: usize) -> Vec<AppEventDto> {
        self.events.lock().map_or_else(
            |_| Vec::new(),
            |events| {
                events
                    .iter()
                    .filter(|event| event.sequence > sequence)
                    .take(limit)
                    .cloned()
                    .collect()
            },
        )
    }
}

fn kernel_event_to_dto(event: KernelEvent) -> AppEventDto {
    let sequence = event.sequence;
    let kind = match event.kind {
        KernelEventKind::BatchSubmitted { batch_id } => AppEventKindDto::BatchSubmitted {
            batch_id: batch_id.as_str().to_owned(),
        },
        KernelEventKind::JobPreparing { batch_id, job_id } => AppEventKindDto::JobPreparing {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
        },
        KernelEventKind::PromptCompiled {
            batch_id,
            job_id,
            expanded_prompt,
        } => AppEventKindDto::PromptCompiled {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            expanded_prompt,
        },
        KernelEventKind::GenerationPlanned {
            batch_id,
            job_id,
            output_mode,
        } => AppEventKindDto::GenerationPlanned {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            output_mode: output_mode_as_str(output_mode).to_owned(),
        },
        KernelEventKind::GenerationStreamChunk {
            batch_id,
            job_id,
            event,
        } => AppEventKindDto::GenerationStreamChunk {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            event_type: event.event_type,
            sample_index: event.sample_index,
            step_index: event.step_index,
            generation_id: event.generation_id,
            sigma: event.sigma,
            image: event.image,
        },
        KernelEventKind::SamplePersisted {
            batch_id,
            job_id,
            sample_index,
            resource,
            artifact_id,
        } => AppEventKindDto::SamplePersisted {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            sample_index,
            resource: resource_ref_to_dto(&resource),
            artifact_id: artifact_id.as_str().to_owned(),
        },
        KernelEventKind::GalleryIndexed {
            batch_id,
            job_id,
            item_id,
        } => AppEventKindDto::GalleryIndexed {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            item_id: item_id.as_str().to_owned(),
        },
        KernelEventKind::SafetyScanFailed {
            batch_id,
            job_id,
            resource,
            message,
        } => AppEventKindDto::SafetyScanFailed {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            resource: resource_ref_to_dto(&resource),
            message,
        },
        KernelEventKind::DirectorSafetyScanFailed {
            run_id,
            resource,
            message,
        } => AppEventKindDto::DirectorSafetyScanFailed {
            run_id,
            resource: resource_ref_to_dto(&resource),
            message,
        },
        KernelEventKind::JobSucceeded { batch_id, job_id } => AppEventKindDto::JobSucceeded {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
        },
        KernelEventKind::JobFailed {
            batch_id,
            job_id,
            message,
            ..
        } => AppEventKindDto::JobFailed {
            batch_id: batch_id.as_str().to_owned(),
            job_id: job_id.as_str().to_owned(),
            message,
        },
    };
    AppEventDto { sequence, kind }
}

const fn output_mode_as_str(mode: GenerationOutputMode) -> &'static str {
    match mode {
        GenerationOutputMode::Image => "image",
        GenerationOutputMode::Stream(_) => "stream",
    }
}

#[cfg(test)]
mod tests {
    use nai_atelier_app_api::event::AppEventKindDto;
    use nai_atelier_jobs::BatchId;
    use nai_atelier_kernel::{KernelEvent, KernelEventKind};
    use nai_atelier_resource_catalog::{ResourceId, ResourceRef};

    use super::{AppEventHub, MAX_RETAINED_EVENTS};

    #[test]
    fn event_hub_keeps_bounded_recent_events() {
        let hub = AppEventHub::default();

        for sequence in 1..=u64::try_from(MAX_RETAINED_EVENTS).unwrap_or(u64::MAX) + 1 {
            hub.push_kernel_event(KernelEvent {
                sequence,
                kind: KernelEventKind::BatchSubmitted {
                    batch_id: BatchId::new(format!("batch-{sequence}")),
                },
            });
        }

        let events = hub.events_since(0, MAX_RETAINED_EVENTS + 2);

        assert_eq!(events.len(), MAX_RETAINED_EVENTS);
        assert_eq!(events[0].sequence, 2);
        assert_eq!(
            events.last().map(|event| event.sequence),
            Some(u64::try_from(MAX_RETAINED_EVENTS).unwrap_or(u64::MAX) + 1)
        );
    }

    #[test]
    fn event_hub_maps_director_safety_scan_failures() {
        let hub = AppEventHub::default();

        hub.push_kernel_event(KernelEvent {
            sequence: 1,
            kind: KernelEventKind::DirectorSafetyScanFailed {
                run_id: "director-1".to_owned(),
                resource: ResourceRef::base(ResourceId::new("resource:director:director-1")),
                message: "scanner unavailable".to_owned(),
            },
        });

        let events = hub.events_since(0, 10);

        assert!(matches!(
            &events[0].kind,
            AppEventKindDto::DirectorSafetyScanFailed { run_id, .. }
                if run_id == "director-1"
        ));
    }
}
