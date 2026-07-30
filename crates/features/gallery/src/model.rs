use atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest,
    ArtifactSource, VisualAssetRef, VisualAssetRole,
};
use atelier_resource_catalog::{ResourceRef, ResourceVariantKind};
use atelier_safety::{SafetyAssessment, SafetyLabel};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GalleryItemId(String);

impl GalleryItemId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn from_artifact_id(id: &ArtifactId) -> Self {
        Self(format!("artifact:{}", id.as_str()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GallerySourceKind {
    Generation,
    Director,
}

impl GallerySourceKind {
    #[must_use]
    pub const fn from_artifact_source(source: &ArtifactSource) -> Self {
        match source {
            ArtifactSource::GenerationJob { .. } => Self::Generation,
            ArtifactSource::DirectorRun { .. } => Self::Director,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GallerySafetyOverride {
    Safe,
    Sensitive,
    Hidden,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GallerySafetyState {
    Unscanned,
    Scanned(Box<SafetyAssessment>),
    Failed {
        message: String,
        attempted_at_ms: u64,
    },
    Unavailable {
        message: String,
    },
}

impl GallerySafetyState {
    #[must_use]
    pub fn assessment(&self) -> Option<&SafetyAssessment> {
        match self {
            Self::Scanned(assessment) => Some(assessment.as_ref()),
            Self::Unscanned | Self::Failed { .. } | Self::Unavailable { .. } => None,
        }
    }
}

impl From<Option<SafetyAssessment>> for GallerySafetyState {
    fn from(assessment: Option<SafetyAssessment>) -> Self {
        assessment.map_or(Self::Unscanned, |assessment| {
            Self::Scanned(Box::new(assessment))
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GalleryItem {
    pub id: GalleryItemId,
    pub artifact_id: ArtifactId,
    pub artifact_kind: ArtifactKind,
    pub source: ArtifactSource,
    pub primary_resource: ResourceRef,
    pub assets: Vec<VisualAssetRef>,
    pub metadata: ArtifactMetadata,
    pub replay: Option<ArtifactReplayManifest>,
    pub safety: GallerySafetyState,
    pub manual_safety_override: Option<GallerySafetyOverride>,
    pub indexed_at_ms: u64,
}

impl GalleryItem {
    #[must_use]
    pub fn from_artifact(
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety: GallerySafetyState,
    ) -> Self {
        let mut metadata = artifact.metadata;
        metadata.embedded_metadata_json = None;
        Self {
            id: GalleryItemId::from_artifact_id(&artifact.id),
            artifact_id: artifact.id,
            artifact_kind: artifact.kind,
            source: artifact.source,
            primary_resource: artifact.primary_resource,
            assets: artifact.assets,
            metadata,
            replay: artifact.replay,
            safety,
            manual_safety_override: None,
            indexed_at_ms,
        }
    }

    #[must_use]
    pub const fn source_kind(&self) -> GallerySourceKind {
        GallerySourceKind::from_artifact_source(&self.source)
    }

    #[must_use]
    pub fn effective_safety_label(&self) -> Option<SafetyLabel> {
        let manual = self.manual_safety_override.map(|value| match value {
            GallerySafetyOverride::Safe => SafetyLabel::Safe,
            GallerySafetyOverride::Sensitive => SafetyLabel::Sensitive,
            GallerySafetyOverride::Hidden => SafetyLabel::Hidden,
        });
        self.safety
            .assessment()
            .map(|assessment| assessment.effective_label(manual))
            .or(manual)
    }

    #[must_use]
    pub fn image_reference(&self, target: ImageReferenceTarget) -> GalleryImageReference {
        let asset = self.preferred_transfer_asset();
        GalleryImageReference {
            item_id: self.id.clone(),
            artifact_id: self.artifact_id.clone(),
            target,
            resource: asset.resource.clone(),
            asset,
        }
    }

    fn preferred_transfer_asset(&self) -> VisualAssetRef {
        self.assets
            .iter()
            .find(|asset| asset.role == VisualAssetRole::Original)
            .or_else(|| {
                self.assets
                    .iter()
                    .find(|asset| asset.role == VisualAssetRole::Sanitized)
            })
            .cloned()
            .unwrap_or_else(|| VisualAssetRef {
                role: VisualAssetRole::Original,
                resource: self.primary_resource.clone(),
                variant_kind: Some(ResourceVariantKind::Original),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryQuery {
    pub offset: usize,
    pub limit: usize,
    pub artifact_kind: Option<ArtifactKind>,
    pub source_kind: Option<GallerySourceKind>,
    pub manual_safety_override: Option<GallerySafetyOverride>,
    pub safety_label: Option<SafetyLabel>,
}

impl Default for GalleryQuery {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 50,
            artifact_kind: None,
            source_kind: None,
            manual_safety_override: None,
            safety_label: None,
        }
    }
}

impl GalleryQuery {
    #[must_use]
    pub fn apply(&self, items: impl IntoIterator<Item = GalleryItem>) -> Vec<GalleryItem> {
        let mut items = items
            .into_iter()
            .filter(|item| self.matches(item))
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .indexed_at_ms
                .cmp(&left.indexed_at_ms)
                .then_with(|| left.id.cmp(&right.id))
        });
        let start = self.offset.min(items.len());
        let end = start.saturating_add(self.limit).min(items.len());
        items[start..end].to_vec()
    }

    fn matches(&self, item: &GalleryItem) -> bool {
        self.artifact_kind
            .is_none_or(|kind| item.artifact_kind == kind)
            && self
                .source_kind
                .is_none_or(|source_kind| item.source_kind() == source_kind)
            && self.matches_manual_safety_override(item)
            && self
                .safety_label
                .is_none_or(|label| item.effective_safety_label() == Some(label))
    }

    fn matches_manual_safety_override(&self, item: &GalleryItem) -> bool {
        if let Some(manual) = self.manual_safety_override {
            return item
                .manual_safety_override
                .is_some_and(|value| value == manual);
        }

        self.safety_label == Some(SafetyLabel::Hidden)
            || item.manual_safety_override != Some(GallerySafetyOverride::Hidden)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageReferenceTarget {
    Director,
    ImageToImage,
    Vibe,
    PreciseReference,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GalleryImageReference {
    pub item_id: GalleryItemId,
    pub artifact_id: ArtifactId,
    pub target: ImageReferenceTarget,
    pub asset: VisualAssetRef,
    pub resource: ResourceRef,
}
