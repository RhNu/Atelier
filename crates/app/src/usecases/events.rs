use super::AppEventDto;
use crate::events::AppEventHub;

pub struct EventsUseCases<'a> {
    pub(crate) events: &'a AppEventHub,
}

impl EventsUseCases<'_> {
    #[must_use]
    pub fn events_since(&self, sequence: u64, limit: usize) -> Vec<AppEventDto> {
        self.events.events_since(sequence, limit)
    }
}
