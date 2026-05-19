use thiserror::Error;

use crate::PromptChunkKey;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PromptResourceErrorKind {
    InvalidRequest,
    NotFound,
    Conflict,
    Repository,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {message}")]
pub struct PromptResourceError {
    kind: PromptResourceErrorKind,
    message: String,
    references: Vec<PromptChunkKey>,
    cycle: Option<PromptFunctionCycle>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptFunctionCycle {
    call_chain: Vec<String>,
}

impl PromptResourceError {
    #[must_use]
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(PromptResourceErrorKind::InvalidRequest, message)
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(PromptResourceErrorKind::NotFound, message)
    }

    #[must_use]
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(PromptResourceErrorKind::Conflict, message)
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self::new(PromptResourceErrorKind::Repository, message)
    }

    #[must_use]
    pub fn cycle_detected(call_chain: Vec<String>) -> Self {
        Self::conflict("prompt function cycle detected").with_cycle(call_chain)
    }

    #[must_use]
    pub fn with_references(mut self, references: Vec<PromptChunkKey>) -> Self {
        self.references = references;
        self
    }

    #[must_use]
    pub fn with_cycle(mut self, call_chain: Vec<String>) -> Self {
        self.cycle = Some(PromptFunctionCycle { call_chain });
        self
    }

    #[must_use]
    pub const fn kind(&self) -> PromptResourceErrorKind {
        self.kind
    }

    #[must_use]
    pub fn references(&self) -> &[PromptChunkKey] {
        &self.references
    }

    #[must_use]
    pub const fn cycle(&self) -> Option<&PromptFunctionCycle> {
        self.cycle.as_ref()
    }

    fn new(kind: PromptResourceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            references: Vec::new(),
            cycle: None,
        }
    }
}

impl PromptFunctionCycle {
    #[must_use]
    pub fn call_chain(&self) -> &[String] {
        &self.call_chain
    }
}

pub type PromptResourceResult<T> = Result<T, PromptResourceError>;
