use super::{
    ArtifactId, ArtifactMetadata, ArtifactRecord, ArtifactReplayManifest, ArtifactSource,
    DatabaseError, DatabaseResult, Deserialize, JSON_SCHEMA_VERSION, JsonCodec, ResourceRefDto,
    Serialize, VisualAssetRef, artifact_kind_as_str, artifact_kind_from_str, ensure_schema,
    variant_kind_as_str, variant_kind_from_str, visual_asset_role_as_str,
    visual_asset_role_from_str,
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
