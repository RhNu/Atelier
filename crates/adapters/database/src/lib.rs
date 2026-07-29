//! `SQLite` database adapters for Atelier.

mod api_keys;
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
mod schema;
mod settings;
mod vibe;

pub use api_keys::DatabaseApiKeyRegistryStore;
pub use artifacts::DatabaseArtifactRepository;
pub use connection::DatabaseConnection;
pub use error::{DatabaseError, DatabaseErrorKind, DatabaseResult};
pub use gallery::{DatabaseGalleryIndex, GalleryHardDeletePlan, GalleryTransientOwner};
pub use generation_draft::DatabaseGenerationDraftRepository;
pub use generation_payload::DatabaseGenerationPayloadStore;
pub use job_history::{DatabaseJobQueueRepository, DatabaseRunHistoryRepository};
pub use prompt_resources::DatabasePromptResourceRepository;
pub use resource_catalog::DatabaseResourceCatalogRepository;
pub use settings::DatabaseSettingsRepository;
pub use vibe::DatabaseVibeRepository;
