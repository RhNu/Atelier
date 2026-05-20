#![allow(clippy::missing_const_for_fn, clippy::struct_field_names)]

use nai_atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest,
    ArtifactSource, VisualAssetRef, VisualAssetRole,
};
use nai_atelier_gallery::{GalleryItem, GalleryItemId, GallerySafetyOverride, GallerySourceKind};
use nai_atelier_resource_catalog::{
    ResourceId, ResourceKind, ResourceLifecycle, ResourceMetadata, ResourceOwnerKind, ResourceRef,
    ResourceRelation, ResourceState, ResourceVariantKind, VariantId,
};
use nai_atelier_safety::{ImageSafetyScore, SafetyAssessment};
use nai_atelier_vibe::{
    VibeDocumentEntry, VibeDocumentResources, VibeDocumentSummary, VibeEncodeSettings,
    VibeEncodingConfig, VibeEncodingRecord, VibeId, VibeModel, VibeSourceIdentity,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{DatabaseError, DatabaseResult};

const JSON_SCHEMA_VERSION: u32 = 1;

pub fn encode_json<T: Serialize>(value: &T) -> DatabaseResult<String> {
    serde_json::to_string(value).map_err(Into::into)
}

pub fn decode_json<T: for<'de> Deserialize<'de>>(text: &str) -> DatabaseResult<T> {
    serde_json::from_str(text).map_err(Into::into)
}

pub trait JsonCodec<T>: DeserializeOwned + Serialize + Sized {
    fn from_domain(value: &T) -> Self;

    fn into_domain(self) -> DatabaseResult<T>;

    fn encode_domain(value: &T) -> DatabaseResult<String> {
        encode_json(&Self::from_domain(value))
    }

    fn decode_domain(text: &str) -> DatabaseResult<T> {
        decode_json::<Self>(text)?.into_domain()
    }
}

pub const fn resource_kind_as_str(value: ResourceKind) -> &'static str {
    match value {
        ResourceKind::GeneratedImage => "generated_image",
        ResourceKind::StreamFinalImage => "stream_final_image",
        ResourceKind::DirectorResult => "director_result",
        ResourceKind::SourceImage => "source_image",
        ResourceKind::ReferenceImage => "reference_image",
        ResourceKind::ControlNetImage => "controlnet_image",
        ResourceKind::PromptThumb => "prompt_thumb",
        ResourceKind::VibeDocument => "vibe_document",
        ResourceKind::VibePreview => "vibe_preview",
        ResourceKind::VibeEncoding => "vibe_encoding",
        ResourceKind::LexiconBundle => "lexicon_bundle",
    }
}

pub fn resource_kind_from_str(value: &str) -> DatabaseResult<ResourceKind> {
    match value {
        "generated_image" => Ok(ResourceKind::GeneratedImage),
        "stream_final_image" => Ok(ResourceKind::StreamFinalImage),
        "director_result" => Ok(ResourceKind::DirectorResult),
        "source_image" => Ok(ResourceKind::SourceImage),
        "reference_image" => Ok(ResourceKind::ReferenceImage),
        "controlnet_image" => Ok(ResourceKind::ControlNetImage),
        "prompt_thumb" => Ok(ResourceKind::PromptThumb),
        "vibe_document" => Ok(ResourceKind::VibeDocument),
        "vibe_preview" => Ok(ResourceKind::VibePreview),
        "vibe_encoding" => Ok(ResourceKind::VibeEncoding),
        "lexicon_bundle" => Ok(ResourceKind::LexiconBundle),
        _ => Err(decode_error("resource kind", value)),
    }
}

pub const fn lifecycle_as_str(value: ResourceLifecycle) -> &'static str {
    match value {
        ResourceLifecycle::WorkspaceScoped => "workspace_scoped",
        ResourceLifecycle::JobScoped => "job_scoped",
        ResourceLifecycle::Cache => "cache",
        ResourceLifecycle::ExportOnly => "export_only",
    }
}

pub fn lifecycle_from_str(value: &str) -> DatabaseResult<ResourceLifecycle> {
    match value {
        "workspace_scoped" => Ok(ResourceLifecycle::WorkspaceScoped),
        "job_scoped" => Ok(ResourceLifecycle::JobScoped),
        "cache" => Ok(ResourceLifecycle::Cache),
        "export_only" => Ok(ResourceLifecycle::ExportOnly),
        _ => Err(decode_error("resource lifecycle", value)),
    }
}

pub const fn resource_state_as_str(value: ResourceState) -> &'static str {
    match value {
        ResourceState::Pending => "pending",
        ResourceState::Ready => "ready",
        ResourceState::DeletePending => "delete_pending",
    }
}

pub fn resource_state_from_str(value: &str) -> DatabaseResult<ResourceState> {
    match value {
        "pending" => Ok(ResourceState::Pending),
        "ready" => Ok(ResourceState::Ready),
        "delete_pending" => Ok(ResourceState::DeletePending),
        _ => Err(decode_error("resource state", value)),
    }
}

pub const fn owner_kind_as_str(value: ResourceOwnerKind) -> &'static str {
    match value {
        ResourceOwnerKind::Job => "job",
        ResourceOwnerKind::GalleryItem => "gallery_item",
        ResourceOwnerKind::PromptResource => "prompt_resource",
        ResourceOwnerKind::Vibe => "vibe",
        ResourceOwnerKind::DirectorRun => "director_run",
        ResourceOwnerKind::Cache => "cache",
        ResourceOwnerKind::ImportStaging => "import_staging",
        ResourceOwnerKind::Workspace => "workspace",
    }
}

pub fn owner_kind_from_str(value: &str) -> DatabaseResult<ResourceOwnerKind> {
    match value {
        "job" => Ok(ResourceOwnerKind::Job),
        "gallery_item" => Ok(ResourceOwnerKind::GalleryItem),
        "prompt_resource" => Ok(ResourceOwnerKind::PromptResource),
        "vibe" => Ok(ResourceOwnerKind::Vibe),
        "director_run" => Ok(ResourceOwnerKind::DirectorRun),
        "cache" => Ok(ResourceOwnerKind::Cache),
        "import_staging" => Ok(ResourceOwnerKind::ImportStaging),
        "workspace" => Ok(ResourceOwnerKind::Workspace),
        _ => Err(decode_error("resource owner kind", value)),
    }
}

pub const fn relation_as_str(value: ResourceRelation) -> &'static str {
    match value {
        ResourceRelation::Primary => "primary",
        ResourceRelation::Source => "source",
        ResourceRelation::Reference => "reference",
        ResourceRelation::Thumbnail => "thumbnail",
        ResourceRelation::Preview => "preview",
        ResourceRelation::Encoding => "encoding",
        ResourceRelation::DerivedFrom => "derived_from",
    }
}

pub fn relation_from_str(value: &str) -> DatabaseResult<ResourceRelation> {
    match value {
        "primary" => Ok(ResourceRelation::Primary),
        "source" => Ok(ResourceRelation::Source),
        "reference" => Ok(ResourceRelation::Reference),
        "thumbnail" => Ok(ResourceRelation::Thumbnail),
        "preview" => Ok(ResourceRelation::Preview),
        "encoding" => Ok(ResourceRelation::Encoding),
        "derived_from" => Ok(ResourceRelation::DerivedFrom),
        _ => Err(decode_error("resource relation", value)),
    }
}

pub const fn variant_kind_as_str(value: ResourceVariantKind) -> &'static str {
    match value {
        ResourceVariantKind::Original => "original",
        ResourceVariantKind::Preview => "preview",
        ResourceVariantKind::Thumbnail => "thumbnail",
        ResourceVariantKind::Sanitized => "sanitized",
        ResourceVariantKind::Export => "export",
    }
}

pub fn variant_kind_from_str(value: &str) -> DatabaseResult<ResourceVariantKind> {
    match value {
        "original" => Ok(ResourceVariantKind::Original),
        "preview" => Ok(ResourceVariantKind::Preview),
        "thumbnail" => Ok(ResourceVariantKind::Thumbnail),
        "sanitized" => Ok(ResourceVariantKind::Sanitized),
        "export" => Ok(ResourceVariantKind::Export),
        _ => Err(decode_error("resource variant kind", value)),
    }
}

pub const fn artifact_kind_as_str(value: ArtifactKind) -> &'static str {
    match value {
        ArtifactKind::GeneratedImage => "generated_image",
        ArtifactKind::DirectorResult => "director_result",
        ArtifactKind::ImportedImage => "imported_image",
    }
}

pub fn artifact_kind_from_str(value: &str) -> DatabaseResult<ArtifactKind> {
    match value {
        "generated_image" => Ok(ArtifactKind::GeneratedImage),
        "director_result" => Ok(ArtifactKind::DirectorResult),
        "imported_image" => Ok(ArtifactKind::ImportedImage),
        _ => Err(decode_error("artifact kind", value)),
    }
}

pub const fn source_kind_as_str(value: GallerySourceKind) -> &'static str {
    match value {
        GallerySourceKind::Generation => "generation",
        GallerySourceKind::Director => "director",
        GallerySourceKind::Import => "import",
    }
}

pub const fn safety_override_as_str(value: GallerySafetyOverride) -> &'static str {
    match value {
        GallerySafetyOverride::Safe => "safe",
        GallerySafetyOverride::Sensitive => "sensitive",
        GallerySafetyOverride::Hidden => "hidden",
    }
}

pub fn safety_override_from_str(value: &str) -> DatabaseResult<GallerySafetyOverride> {
    match value {
        "safe" => Ok(GallerySafetyOverride::Safe),
        "sensitive" => Ok(GallerySafetyOverride::Sensitive),
        "hidden" => Ok(GallerySafetyOverride::Hidden),
        _ => Err(decode_error("gallery safety override", value)),
    }
}

pub const fn vibe_model_as_str(value: VibeModel) -> &'static str {
    value.vibe_model_key()
}

pub fn vibe_model_from_str(value: &str) -> DatabaseResult<VibeModel> {
    VibeModel::from_vibe_model_key(value).ok_or_else(|| decode_error("vibe model", value))
}

pub fn metadata_from_columns(
    mime_type: Option<String>,
    byte_size: Option<u64>,
    content_hash: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    created_at_ms: Option<u64>,
) -> ResourceMetadata {
    ResourceMetadata {
        mime_type,
        byte_size,
        content_hash,
        width,
        height,
        created_at_ms,
    }
}

pub fn decode_error(kind: &str, value: &str) -> DatabaseError {
    DatabaseError::new(format!("unknown {kind} `{value}`"))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceRefDto {
    id: String,
    variant_id: Option<String>,
}

impl From<&ResourceRef> for ResourceRefDto {
    fn from(value: &ResourceRef) -> Self {
        Self {
            id: value.id.as_str().to_owned(),
            variant_id: value.variant_id.as_ref().map(|id| id.as_str().to_owned()),
        }
    }
}

impl ResourceRefDto {
    pub fn into_domain(self) -> ResourceRef {
        ResourceRef::new(
            ResourceId::new(self.id),
            self.variant_id.map(VariantId::new),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VisualAssetRefDto {
    role: String,
    resource: ResourceRefDto,
    variant_kind: Option<String>,
}

impl From<&VisualAssetRef> for VisualAssetRefDto {
    fn from(value: &VisualAssetRef) -> Self {
        Self {
            role: visual_asset_role_as_str(value.role).to_owned(),
            resource: ResourceRefDto::from(&value.resource),
            variant_kind: value
                .variant_kind
                .map(variant_kind_as_str)
                .map(str::to_owned),
        }
    }
}

impl VisualAssetRefDto {
    fn into_domain(self) -> DatabaseResult<VisualAssetRef> {
        Ok(VisualAssetRef {
            role: visual_asset_role_from_str(&self.role)?,
            resource: self.resource.into_domain(),
            variant_kind: self
                .variant_kind
                .as_deref()
                .map(variant_kind_from_str)
                .transpose()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ArtifactMetadataDto {
    seed: Option<i64>,
    sample_index: Option<u32>,
    model_name: Option<String>,
    extensions: std::collections::BTreeMap<String, String>,
}

impl From<&ArtifactMetadata> for ArtifactMetadataDto {
    fn from(value: &ArtifactMetadata) -> Self {
        Self {
            seed: value.seed,
            sample_index: value.sample_index,
            model_name: value.model_name.clone(),
            extensions: value.extensions.clone(),
        }
    }
}

impl From<ArtifactMetadataDto> for ArtifactMetadata {
    fn from(value: ArtifactMetadataDto) -> Self {
        Self {
            seed: value.seed,
            sample_index: value.sample_index,
            model_name: value.model_name,
            extensions: value.extensions,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ArtifactReplayManifestDto {
    payload_ref: Option<String>,
    prepared_payload_ref: Option<String>,
    prompt_snapshot: Option<String>,
    negative_prompt_snapshot: Option<String>,
}

impl From<&ArtifactReplayManifest> for ArtifactReplayManifestDto {
    fn from(value: &ArtifactReplayManifest) -> Self {
        Self {
            payload_ref: value.payload_ref.clone(),
            prepared_payload_ref: value.prepared_payload_ref.clone(),
            prompt_snapshot: value.prompt_snapshot.clone(),
            negative_prompt_snapshot: value.negative_prompt_snapshot.clone(),
        }
    }
}

impl From<ArtifactReplayManifestDto> for ArtifactReplayManifest {
    fn from(value: ArtifactReplayManifestDto) -> Self {
        Self {
            payload_ref: value.payload_ref,
            prepared_payload_ref: value.prepared_payload_ref,
            prompt_snapshot: value.prompt_snapshot,
            negative_prompt_snapshot: value.negative_prompt_snapshot,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
struct ArtifactSourceDto {
    job_id: Option<String>,
    batch_id: Option<String>,
    run_id: Option<String>,
    import_id: Option<String>,
}

impl From<&ArtifactSource> for ArtifactSourceDto {
    fn from(value: &ArtifactSource) -> Self {
        match value {
            ArtifactSource::GenerationJob { job_id, batch_id } => Self {
                job_id: Some(job_id.clone()),
                batch_id: batch_id.clone(),
                run_id: None,
                import_id: None,
            },
            ArtifactSource::DirectorRun { run_id } => Self {
                job_id: None,
                batch_id: None,
                run_id: Some(run_id.clone()),
                import_id: None,
            },
            ArtifactSource::Import { import_id } => Self {
                job_id: None,
                batch_id: None,
                run_id: None,
                import_id: Some(import_id.clone()),
            },
        }
    }
}

impl ArtifactSourceDto {
    fn into_domain(self) -> DatabaseResult<ArtifactSource> {
        if let Some(job_id) = self.job_id {
            return Ok(ArtifactSource::GenerationJob {
                job_id,
                batch_id: self.batch_id,
            });
        }
        if let Some(run_id) = self.run_id {
            return Ok(ArtifactSource::DirectorRun { run_id });
        }
        if let Some(import_id) = self.import_id {
            return Ok(ArtifactSource::Import { import_id });
        }
        Err(DatabaseError::new("artifact source is missing a source id"))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArtifactRecordDto {
    schema_version: u32,
    id: String,
    kind: String,
    source: ArtifactSourceDto,
    primary_resource: ResourceRefDto,
    metadata: ArtifactMetadataDto,
    replay: Option<ArtifactReplayManifestDto>,
    assets: Vec<VisualAssetRefDto>,
}

impl JsonCodec<ArtifactRecord> for ArtifactRecordDto {
    fn from_domain(value: &ArtifactRecord) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            id: value.id.as_str().to_owned(),
            kind: artifact_kind_as_str(value.kind).to_owned(),
            source: ArtifactSourceDto::from(&value.source),
            primary_resource: ResourceRefDto::from(&value.primary_resource),
            metadata: ArtifactMetadataDto::from(&value.metadata),
            replay: value.replay.as_ref().map(ArtifactReplayManifestDto::from),
            assets: value.assets.iter().map(VisualAssetRefDto::from).collect(),
        }
    }

    fn into_domain(self) -> DatabaseResult<ArtifactRecord> {
        ensure_schema(self.schema_version)?;
        Ok(ArtifactRecord {
            id: ArtifactId::new(self.id),
            kind: artifact_kind_from_str(&self.kind)?,
            source: self.source.into_domain()?,
            primary_resource: self.primary_resource.into_domain(),
            metadata: self.metadata.into(),
            replay: self.replay.map(Into::into),
            assets: self
                .assets
                .into_iter()
                .map(VisualAssetRefDto::into_domain)
                .collect::<DatabaseResult<Vec<_>>>()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SafetyAssessmentDto {
    resource: ResourceRefDto,
    score: f32,
    scorer_label: Option<String>,
    scorer_version: Option<String>,
    assessed_at_ms: Option<u64>,
}

impl From<&SafetyAssessment> for SafetyAssessmentDto {
    fn from(value: &SafetyAssessment) -> Self {
        Self {
            resource: ResourceRefDto::from(&value.resource),
            score: value.score.value(),
            scorer_label: value.scorer_label.clone(),
            scorer_version: value.scorer_version.clone(),
            assessed_at_ms: value.assessed_at_ms,
        }
    }
}

impl SafetyAssessmentDto {
    fn into_domain(self) -> DatabaseResult<SafetyAssessment> {
        Ok(SafetyAssessment {
            resource: self.resource.into_domain(),
            score: ImageSafetyScore::new(self.score)
                .map_err(|error| DatabaseError::new(error.to_string()))?,
            scorer_label: self.scorer_label,
            scorer_version: self.scorer_version,
            assessed_at_ms: self.assessed_at_ms,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GalleryItemDto {
    schema_version: u32,
    id: String,
    artifact_id: String,
    artifact_kind: String,
    source: ArtifactSourceDto,
    primary_resource: ResourceRefDto,
    assets: Vec<VisualAssetRefDto>,
    metadata: ArtifactMetadataDto,
    safety_assessment: Option<SafetyAssessmentDto>,
    manual_safety_override: Option<String>,
    indexed_at_ms: u64,
}

impl JsonCodec<GalleryItem> for GalleryItemDto {
    fn from_domain(value: &GalleryItem) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            id: value.id.as_str().to_owned(),
            artifact_id: value.artifact_id.as_str().to_owned(),
            artifact_kind: artifact_kind_as_str(value.artifact_kind).to_owned(),
            source: ArtifactSourceDto::from(&value.source),
            primary_resource: ResourceRefDto::from(&value.primary_resource),
            assets: value.assets.iter().map(VisualAssetRefDto::from).collect(),
            metadata: ArtifactMetadataDto::from(&value.metadata),
            safety_assessment: value
                .safety_assessment
                .as_ref()
                .map(SafetyAssessmentDto::from),
            manual_safety_override: value
                .manual_safety_override
                .map(safety_override_as_str)
                .map(str::to_owned),
            indexed_at_ms: value.indexed_at_ms,
        }
    }

    fn into_domain(self) -> DatabaseResult<GalleryItem> {
        ensure_schema(self.schema_version)?;
        Ok(GalleryItem {
            id: GalleryItemId::new(self.id),
            artifact_id: ArtifactId::new(self.artifact_id),
            artifact_kind: artifact_kind_from_str(&self.artifact_kind)?,
            source: self.source.into_domain()?,
            primary_resource: self.primary_resource.into_domain(),
            assets: self
                .assets
                .into_iter()
                .map(VisualAssetRefDto::into_domain)
                .collect::<DatabaseResult<Vec<_>>>()?,
            metadata: self.metadata.into(),
            safety_assessment: self
                .safety_assessment
                .map(SafetyAssessmentDto::into_domain)
                .transpose()?,
            manual_safety_override: self
                .manual_safety_override
                .as_deref()
                .map(safety_override_from_str)
                .transpose()?,
            indexed_at_ms: self.indexed_at_ms,
        })
    }
}

const fn visual_asset_role_as_str(value: VisualAssetRole) -> &'static str {
    match value {
        VisualAssetRole::Original => "original",
        VisualAssetRole::Thumbnail => "thumbnail",
        VisualAssetRole::Preview => "preview",
        VisualAssetRole::Sanitized => "sanitized",
        VisualAssetRole::Export => "export",
    }
}

fn visual_asset_role_from_str(value: &str) -> DatabaseResult<VisualAssetRole> {
    match value {
        "original" => Ok(VisualAssetRole::Original),
        "thumbnail" => Ok(VisualAssetRole::Thumbnail),
        "preview" => Ok(VisualAssetRole::Preview),
        "sanitized" => Ok(VisualAssetRole::Sanitized),
        "export" => Ok(VisualAssetRole::Export),
        _ => Err(decode_error("visual asset role", value)),
    }
}

fn ensure_schema(version: u32) -> DatabaseResult<()> {
    if version == JSON_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(DatabaseError::new(format!(
            "unsupported JSON schema version {version}"
        )))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VibeEncodeSettingsDto {
    model: String,
    information_extracted: f32,
}

impl From<&VibeEncodeSettings> for VibeEncodeSettingsDto {
    fn from(value: &VibeEncodeSettings) -> Self {
        Self {
            model: vibe_model_as_str(value.model).to_owned(),
            information_extracted: value.information_extracted,
        }
    }
}

impl VibeEncodeSettingsDto {
    fn into_domain(self) -> DatabaseResult<VibeEncodeSettings> {
        VibeEncodeSettings::new(
            vibe_model_from_str(&self.model)?,
            self.information_extracted,
        )
        .map_err(|error| DatabaseError::new(error.to_string()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VibeEncodingConfigDto {
    model: String,
    settings: VibeEncodeSettingsDto,
}

impl From<&VibeEncodingConfig> for VibeEncodingConfigDto {
    fn from(value: &VibeEncodingConfig) -> Self {
        Self {
            model: vibe_model_as_str(value.model).to_owned(),
            settings: VibeEncodeSettingsDto::from(&value.settings),
        }
    }
}

impl VibeEncodingConfigDto {
    fn into_domain(self) -> DatabaseResult<VibeEncodingConfig> {
        Ok(VibeEncodingConfig {
            model: vibe_model_from_str(&self.model)?,
            settings: self.settings.into_domain()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VibeDocumentSummaryDto {
    document_id: String,
    display_name: String,
    has_image: bool,
    available_model_keys: Vec<String>,
    available_encoding_configs: Vec<VibeEncodingConfigDto>,
}

impl From<&VibeDocumentSummary> for VibeDocumentSummaryDto {
    fn from(value: &VibeDocumentSummary) -> Self {
        Self {
            document_id: value.document_id.as_str().to_owned(),
            display_name: value.display_name.clone(),
            has_image: value.has_image,
            available_model_keys: value.available_model_keys.clone(),
            available_encoding_configs: value
                .available_encoding_configs
                .iter()
                .map(VibeEncodingConfigDto::from)
                .collect(),
        }
    }
}

impl VibeDocumentSummaryDto {
    fn into_domain(self) -> DatabaseResult<VibeDocumentSummary> {
        Ok(VibeDocumentSummary {
            document_id: VibeId::new(self.document_id),
            display_name: self.display_name,
            has_image: self.has_image,
            available_model_keys: self.available_model_keys,
            available_encoding_configs: self
                .available_encoding_configs
                .into_iter()
                .map(VibeEncodingConfigDto::into_domain)
                .collect::<DatabaseResult<Vec<_>>>()?,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct VibeDocumentResourcesDto {
    document: ResourceRefDto,
    source_image: Option<ResourceRefDto>,
    preview: Option<ResourceRefDto>,
    encodings: Vec<ResourceRefDto>,
}

impl From<&VibeDocumentResources> for VibeDocumentResourcesDto {
    fn from(value: &VibeDocumentResources) -> Self {
        Self {
            document: ResourceRefDto::from(&value.document),
            source_image: value.source_image.as_ref().map(ResourceRefDto::from),
            preview: value.preview.as_ref().map(ResourceRefDto::from),
            encodings: value.encodings.iter().map(ResourceRefDto::from).collect(),
        }
    }
}

impl VibeDocumentResourcesDto {
    fn into_domain(self) -> VibeDocumentResources {
        VibeDocumentResources {
            document: self.document.into_domain(),
            source_image: self.source_image.map(ResourceRefDto::into_domain),
            preview: self.preview.map(ResourceRefDto::into_domain),
            encodings: self
                .encodings
                .into_iter()
                .map(ResourceRefDto::into_domain)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VibeDocumentEntryDto {
    schema_version: u32,
    summary: VibeDocumentSummaryDto,
    resources: VibeDocumentResourcesDto,
}

impl JsonCodec<VibeDocumentEntry> for VibeDocumentEntryDto {
    fn from_domain(value: &VibeDocumentEntry) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            summary: VibeDocumentSummaryDto::from(&value.summary),
            resources: VibeDocumentResourcesDto::from(&value.resources),
        }
    }

    fn into_domain(self) -> DatabaseResult<VibeDocumentEntry> {
        ensure_schema(self.schema_version)?;
        Ok(VibeDocumentEntry {
            summary: self.summary.into_domain()?,
            resources: self.resources.into_domain(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VibeEncodingRecordDto {
    schema_version: u32,
    vibe_id: String,
    source_hash: String,
    settings: VibeEncodeSettingsDto,
    resource: ResourceRefDto,
}

impl JsonCodec<VibeEncodingRecord> for VibeEncodingRecordDto {
    fn from_domain(value: &VibeEncodingRecord) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            vibe_id: value.vibe_id.as_str().to_owned(),
            source_hash: value.source.content_hash.clone(),
            settings: VibeEncodeSettingsDto::from(&value.settings),
            resource: ResourceRefDto::from(&value.resource),
        }
    }

    fn into_domain(self) -> DatabaseResult<VibeEncodingRecord> {
        ensure_schema(self.schema_version)?;
        Ok(VibeEncodingRecord {
            vibe_id: VibeId::new(self.vibe_id),
            source: VibeSourceIdentity::new_sha256(self.source_hash),
            settings: self.settings.into_domain()?,
            resource: self.resource.into_domain(),
        })
    }
}
