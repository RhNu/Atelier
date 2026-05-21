use atelier_app_api::event::{AppEventPageDto, EventsSinceRequestDto};

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E> {
    /// Returns app events after the requested sequence.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open.
    pub fn events_since(&self, request: EventsSinceRequestDto) -> CommandResult<AppEventPageDto> {
        let items = self
            .current_app()?
            .events()
            .events_since(request.sequence, request.limit);
        let next_sequence = items
            .last()
            .map_or(request.sequence, |event| event.sequence);
        Ok(AppEventPageDto {
            items,
            next_sequence,
        })
    }
}
