use super::{
    ArtifactId, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest, ArtifactSource,
    DatabaseError, DatabaseResult, Deserialize, EmbeddedMetadataStatus, EmbeddedMetadataWarning,
    JSON_SCHEMA_VERSION, JsonCodec, ResourceRefDto, Serialize, VisualAssetRef,
    artifact_kind_as_str, artifact_kind_from_str, ensure_schema, variant_kind_as_str,
    variant_kind_from_str, visual_asset_role_as_str, visual_asset_role_from_str,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct VisualAssetRefDto {
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
    pub(super) fn into_domain(self) -> DatabaseResult<VisualAssetRef> {
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
pub(super) struct ArtifactMetadataDto {
    #[serde(default)]
    request_seed: Option<i64>,
    seed: Option<i64>,
    sample_index: Option<u32>,
    model_name: Option<String>,
    #[serde(default)]
    embedded_metadata_status: Option<String>,
    #[serde(default)]
    embedded_prompt: Option<String>,
    #[serde(default)]
    embedded_negative_prompt: Option<String>,
    #[serde(default)]
    embedded_metadata_json: Option<String>,
    #[serde(default)]
    embedded_metadata_error: Option<String>,
    #[serde(default)]
    embedded_metadata_warnings: Vec<EmbeddedMetadataWarningDto>,
    extensions: std::collections::BTreeMap<String, String>,
}

impl From<&ArtifactMetadata> for ArtifactMetadataDto {
    fn from(value: &ArtifactMetadata) -> Self {
        Self {
            request_seed: value.request_seed,
            seed: value.seed,
            sample_index: value.sample_index,
            model_name: value.model_name.clone(),
            embedded_metadata_status: value
                .embedded_metadata_status
                .map(embedded_metadata_status_as_str)
                .map(str::to_owned),
            embedded_prompt: value.embedded_prompt.clone(),
            embedded_negative_prompt: value.embedded_negative_prompt.clone(),
            embedded_metadata_json: value.embedded_metadata_json.clone(),
            embedded_metadata_error: value.embedded_metadata_error.clone(),
            embedded_metadata_warnings: value
                .embedded_metadata_warnings
                .iter()
                .map(EmbeddedMetadataWarningDto::from)
                .collect(),
            extensions: value.extensions.clone(),
        }
    }
}

impl From<ArtifactMetadataDto> for ArtifactMetadata {
    fn from(value: ArtifactMetadataDto) -> Self {
        Self {
            request_seed: value.request_seed,
            seed: value.seed,
            sample_index: value.sample_index,
            model_name: value.model_name,
            embedded_metadata_status: value
                .embedded_metadata_status
                .as_deref()
                .map(embedded_metadata_status_from_str),
            embedded_prompt: value.embedded_prompt,
            embedded_negative_prompt: value.embedded_negative_prompt,
            embedded_metadata_json: value.embedded_metadata_json,
            embedded_metadata_error: value.embedded_metadata_error,
            embedded_metadata_warnings: value
                .embedded_metadata_warnings
                .into_iter()
                .map(EmbeddedMetadataWarningDto::into_domain)
                .collect(),
            extensions: value.extensions,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct EmbeddedMetadataWarningDto {
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyword: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl From<&EmbeddedMetadataWarning> for EmbeddedMetadataWarningDto {
    fn from(value: &EmbeddedMetadataWarning) -> Self {
        match value {
            EmbeddedMetadataWarning::InvalidCommentJson => Self {
                code: "invalid_comment_json".to_owned(),
                keyword: None,
                message: None,
            },
            EmbeddedMetadataWarning::InvalidTextChunk { keyword, message } => Self {
                code: "invalid_text_chunk".to_owned(),
                keyword: Some(keyword.clone()),
                message: Some(message.clone()),
            },
            EmbeddedMetadataWarning::Unknown(value) => Self {
                code: "unknown".to_owned(),
                keyword: None,
                message: Some(value.clone()),
            },
        }
    }
}

impl EmbeddedMetadataWarningDto {
    fn into_domain(self) -> EmbeddedMetadataWarning {
        if self.code == "invalid_comment_json" {
            EmbeddedMetadataWarning::InvalidCommentJson
        } else if self.code == "invalid_text_chunk" {
            EmbeddedMetadataWarning::InvalidTextChunk {
                keyword: self.keyword.unwrap_or_default(),
                message: self.message.unwrap_or_default(),
            }
        } else if self.code == "unknown" {
            EmbeddedMetadataWarning::Unknown(self.message.unwrap_or_default())
        } else {
            EmbeddedMetadataWarning::Unknown(self.code)
        }
    }
}

const fn embedded_metadata_status_as_str(value: EmbeddedMetadataStatus) -> &'static str {
    match value {
        EmbeddedMetadataStatus::Parsed => "parsed",
        EmbeddedMetadataStatus::NotPresent => "not_present",
        EmbeddedMetadataStatus::UnsupportedFormat => "unsupported_format",
        EmbeddedMetadataStatus::Invalid => "invalid",
    }
}

fn embedded_metadata_status_from_str(value: &str) -> EmbeddedMetadataStatus {
    match value {
        "parsed" => EmbeddedMetadataStatus::Parsed,
        "not_present" => EmbeddedMetadataStatus::NotPresent,
        "unsupported_format" => EmbeddedMetadataStatus::UnsupportedFormat,
        _ => EmbeddedMetadataStatus::Invalid,
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ArtifactReplayManifestDto {
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
pub(super) struct ArtifactSourceDto {
    job_id: Option<String>,
    batch_id: Option<String>,
    run_id: Option<String>,
}

impl From<&ArtifactSource> for ArtifactSourceDto {
    fn from(value: &ArtifactSource) -> Self {
        match value {
            ArtifactSource::GenerationJob { job_id, batch_id } => Self {
                job_id: Some(job_id.clone()),
                batch_id: batch_id.clone(),
                run_id: None,
            },
            ArtifactSource::DirectorRun { run_id } => Self {
                job_id: None,
                batch_id: None,
                run_id: Some(run_id.clone()),
            },
        }
    }
}

impl ArtifactSourceDto {
    pub(super) fn into_domain(self) -> DatabaseResult<ArtifactSource> {
        if let Some(job_id) = self.job_id {
            return Ok(ArtifactSource::GenerationJob {
                job_id,
                batch_id: self.batch_id,
            });
        }
        if let Some(run_id) = self.run_id {
            return Ok(ArtifactSource::DirectorRun { run_id });
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
