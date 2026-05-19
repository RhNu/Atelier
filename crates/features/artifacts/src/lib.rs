//! Artifact domain model and registration rules for persisted visual outputs.

mod error;
mod model;
mod ports;
mod service;

pub use error::{ArtifactError, ArtifactErrorKind, ArtifactResult};
pub use model::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest,
    ArtifactSource, RegisterArtifactRequest, VisualAssetRef, VisualAssetRole,
};
pub use ports::{ArtifactRepository, ArtifactResourceReader};
pub use service::ArtifactService;
