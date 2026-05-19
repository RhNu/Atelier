use std::collections::BTreeMap;

use nai_atelier_resource_catalog::{ResourceKind, ResourceRef, ResourceVariantKind};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactId(String);

impl ArtifactId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    GeneratedImage,
    DirectorResult,
    ImportedImage,
}

impl ArtifactKind {
    #[must_use]
    pub const fn accepts_resource_kind(self, resource_kind: ResourceKind) -> bool {
        match self {
            Self::GeneratedImage => matches!(
                resource_kind,
                ResourceKind::GeneratedImage | ResourceKind::StreamFinalImage
            ),
            Self::DirectorResult => matches!(resource_kind, ResourceKind::DirectorResult),
            Self::ImportedImage => matches!(
                resource_kind,
                ResourceKind::SourceImage
                    | ResourceKind::ReferenceImage
                    | ResourceKind::ControlNetImage
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactSource {
    GenerationJob {
        job_id: String,
        batch_id: Option<String>,
    },
    DirectorRun {
        run_id: String,
    },
    Import {
        import_id: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactMetadata {
    pub seed: Option<i64>,
    pub sample_index: Option<u32>,
    pub model_name: Option<String>,
    pub extensions: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactReplayManifest {
    pub payload_ref: Option<String>,
    pub prepared_payload_ref: Option<String>,
    pub prompt_snapshot: Option<String>,
    pub negative_prompt_snapshot: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VisualAssetRole {
    Original,
    Preview,
    Sanitized,
    Export,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualAssetRef {
    pub role: VisualAssetRole,
    pub resource: ResourceRef,
    pub variant_kind: Option<ResourceVariantKind>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisterArtifactRequest {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub source: ArtifactSource,
    pub primary_resource: ResourceRef,
    pub metadata: ArtifactMetadata,
    pub replay: Option<ArtifactReplayManifest>,
    pub assets: Vec<VisualAssetRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub source: ArtifactSource,
    pub primary_resource: ResourceRef,
    pub metadata: ArtifactMetadata,
    pub replay: Option<ArtifactReplayManifest>,
    pub assets: Vec<VisualAssetRef>,
}
