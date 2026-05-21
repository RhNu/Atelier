use super::{AppEventDto, AtelierApp};

pub struct EventsUseCases<'a, S, F, E> {
    pub(crate) app: &'a AtelierApp<S, F, E>,
}

impl<S, F, E> EventsUseCases<'_, S, F, E> {
    #[must_use]
    pub fn events_since(&self, sequence: u64, limit: usize) -> Vec<AppEventDto> {
        self.app.inner.events.events_since(sequence, limit)
    }
}
