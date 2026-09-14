//! `SQLite` database adapters for Atelier.

mod agent;
mod agent_edits;
mod artifacts;
mod codec;
mod connection;
mod error;
mod gallery;
pub(crate) mod generation_codec;
mod generation_draft;
mod generation_payload;
mod job_history;
mod prompt_resources;
mod resource_catalog;
mod resource_library;
mod schema;
mod settings;
mod vibe;

pub use agent::DatabaseAgentWorkspaceRepository;
pub use agent_edits::DatabaseAgentEditStore;
pub use artifacts::DatabaseArtifactRepository;
pub use connection::DatabaseConnection;
pub use error::{DatabaseError, DatabaseErrorKind, DatabaseResult};
pub use gallery::{DatabaseGalleryIndex, GalleryHardDeletePlan, GalleryTransientOwner};
pub use generation_draft::DatabaseGenerationDraftRepository;
pub use generation_payload::DatabaseGenerationPayloadStore;
pub use job_history::{DatabaseGenerationStore, DatabaseRunHistoryRepository};
pub use prompt_resources::DatabasePromptResourceRepository;
pub use resource_catalog::DatabaseResourceCatalogRepository;
pub use resource_library::DatabaseResourceLibraryRepository;
pub use settings::DatabaseSettingsRepository;
pub use vibe::DatabaseVibeRepository;
