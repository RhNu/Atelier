use super::{
    ArtifactKind, DatabaseError, DatabaseResult, GallerySafetyOverride, GallerySourceKind,
    ResourceKind, ResourceLifecycle, ResourceMetadata, ResourceOwnerKind, ResourceRelation,
    ResourceState, ResourceVariantKind, VibeModel,
};

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
