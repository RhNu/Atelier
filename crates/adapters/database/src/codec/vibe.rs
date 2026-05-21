use super::{
    DatabaseError, DatabaseResult, Deserialize, JSON_SCHEMA_VERSION, JsonCodec, ResourceRefDto,
    Serialize, VibeDocumentEntry, VibeDocumentResources, VibeDocumentSummary, VibeEncodeSettings,
    VibeEncodingConfig, VibeEncodingRecord, VibeId, VibeSourceIdentity, ensure_schema,
    vibe_model_as_str, vibe_model_from_str,
};

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
