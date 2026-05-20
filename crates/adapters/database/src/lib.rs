//! `SQLite` database adapters for NAI Atelier.

mod api_keys;
mod artifacts;
mod codec;
mod connection;
mod error;
mod gallery;
mod generation_codec;
mod generation_payload;
mod migration;
mod resource_catalog;
mod vibe;

pub use api_keys::DatabaseApiKeyRegistryStore;
pub use artifacts::DatabaseArtifactRepository;
pub use connection::DatabaseConnection;
pub use error::{DatabaseError, DatabaseResult};
pub use gallery::DatabaseGalleryIndex;
pub use generation_payload::DatabaseGenerationPayloadStore;
pub use resource_catalog::DatabaseResourceCatalogRepository;
pub use vibe::DatabaseVibeRepository;
