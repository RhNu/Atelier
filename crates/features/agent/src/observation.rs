/// Tracks state actually delivered to a model, separately from results still being executed.
/// The host promotes pending observations only when starting the next model request.
#[derive(Clone, Debug)]
pub struct AgentObservation<T> {
    known: Option<T>,
    pending: Option<T>,
}

impl<T> AgentObservation<T> {
    #[must_use]
    pub const fn new(initial: Option<T>) -> Self {
        Self {
            known: initial,
            pending: None,
        }
    }

    #[must_use]
    pub const fn known(&self) -> Option<&T> {
        self.known.as_ref()
    }

    pub fn publish(&mut self, value: T) {
        self.pending = Some(value);
    }

    pub fn invalidate(&mut self) {
        self.known = None;
    }

    pub fn begin_model_step(&mut self) {
        if let Some(value) = self.pending.take() {
            self.known = Some(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AgentObservation;

    #[test]
    fn results_do_not_authorize_already_queued_edits() {
        let mut state = AgentObservation::new(Some("old"));
        state.publish("new");
        assert_eq!(state.known(), Some(&"old"));
        state.invalidate();
        assert_eq!(state.known(), None);
        state.begin_model_step();
        assert_eq!(state.known(), Some(&"new"));
    }

    #[test]
    fn unread_objects_have_no_edit_baseline() {
        let mut state = AgentObservation::new(None::<u32>);
        state.begin_model_step();
        assert_eq!(state.known(), None);
    }
}
