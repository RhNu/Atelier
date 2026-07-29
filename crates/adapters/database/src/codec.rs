#![allow(clippy::missing_const_for_fn, clippy::struct_field_names)]

use atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest,
    ArtifactSource, EmbeddedMetadataStatus, EmbeddedMetadataWarning, VisualAssetRef,
    VisualAssetRole,
};
use atelier_gallery::{GalleryItem, GalleryItemId, GallerySafetyOverride, GallerySourceKind};
use atelier_resource_catalog::{
    ResourceId, ResourceKind, ResourceLifecycle, ResourceMetadata, ResourceOwnerKind, ResourceRef,
    ResourceRelation, ResourceState, ResourceVariantKind, VariantId,
};
use atelier_safety::{ImageSafetyScore, SafetyAssessment, SafetyModelScore};
use atelier_vibe::{
    VibeDocumentEntry, VibeDocumentResources, VibeDocumentSummary, VibeEncodeSettings,
    VibeEncodingConfig, VibeEncodingRecord, VibeId, VibeModel, VibeSourceIdentity,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{DatabaseError, DatabaseResult};

mod artifact;
mod gallery;
mod json;
mod resource;
mod scalars;
mod vibe;

use artifact::{
    ArtifactMetadataDto, ArtifactReplayManifestDto, ArtifactSourceDto, VisualAssetRefDto,
};
use gallery::{visual_asset_role_as_str, visual_asset_role_from_str};

pub use artifact::ArtifactRecordDto;
pub use gallery::GalleryItemDto;
pub use json::{JsonCodec, decode_json, encode_json};
pub use resource::ResourceRefDto;
pub use scalars::*;
pub use vibe::{VibeDocumentEntryDto, VibeEncodingRecordDto};

const JSON_SCHEMA_VERSION: u32 = 1;

fn ensure_schema(version: u32) -> DatabaseResult<()> {
    if version == JSON_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(DatabaseError::new(format!(
            "unsupported JSON schema version {version}"
        )))
    }
}
