//! Host-neutral application use cases for Atelier.

mod app;
mod commands;
mod error;
mod events;
mod mapping;
mod ports;
mod usecases;
mod worker;

pub use app::WorkspaceSession;
pub use commands::{AtelierRuntime, CommandResult};
pub use error::{AppError, AppResult};
pub use events::{AppEventHub, AppEventListener};
pub use worker::GenerationWorkerCancel;

/// Preloads process-wide static resources used by workspace sessions.
///
/// # Errors
/// Returns an error when an embedded static resource is invalid.
pub fn preload_static_resources() -> AppResult<()> {
    let lexicon =
        atelier_prompt_lexicon::PromptLexicon::load_embedded_shared().map_err(AppError::from)?;
    lexicon.warm_search_index();
    Ok(())
}
