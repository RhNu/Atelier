use super::{
    EnsuredVibeEncoding, EnsuredVibeEncodingDto, ExportedVibeDocument, ExportedVibeDocumentDto,
    ImportedVibeDocuments, ImportedVibeDocumentsDto, VibeDocumentEntry, VibeDocumentEntryDto,
    VibeEncodingConfigDto, VibeExportFormat, VibeExportFormatDto, VibeModel, VibeModelDto,
    resource_ref_to_dto,
};

pub fn imported_vibes_to_dto(value: ImportedVibeDocuments) -> ImportedVibeDocumentsDto {
    ImportedVibeDocumentsDto {
        entries: value.entries.into_iter().map(vibe_entry_to_dto).collect(),
    }
}

pub fn exported_vibe_to_dto(value: ExportedVibeDocument) -> ExportedVibeDocumentDto {
    ExportedVibeDocumentDto {
        file_extension: value.document.file_extension.to_owned(),
        content: value.document.content,
    }
}

pub fn ensured_vibe_to_dto(value: &EnsuredVibeEncoding) -> EnsuredVibeEncodingDto {
    EnsuredVibeEncodingDto {
        resource: resource_ref_to_dto(&value.record.resource),
        created: value.created,
    }
}

pub fn vibe_entry_to_dto(value: VibeDocumentEntry) -> VibeDocumentEntryDto {
    VibeDocumentEntryDto {
        vibe_id: value.summary.document_id.as_str().to_owned(),
        display_name: value.summary.display_name,
        has_image: value.summary.has_image,
        hidden: value.summary.hidden,
        available_model_keys: value.summary.available_model_keys,
        available_encoding_configs: value
            .summary
            .available_encoding_configs
            .into_iter()
            .map(|config| VibeEncodingConfigDto {
                model: vibe_model_to_dto(config.model),
                information_extracted: config.settings.normalized_information_extracted(),
            })
            .collect(),
        created_at_ms: value.summary.created_at_ms,
        updated_at_ms: value.summary.updated_at_ms,
        document: resource_ref_to_dto(&value.resources.document),
        source_image: value
            .resources
            .source_image
            .as_ref()
            .map(resource_ref_to_dto),
        preview: value.resources.preview.as_ref().map(resource_ref_to_dto),
        encodings: value
            .resources
            .encodings
            .iter()
            .map(resource_ref_to_dto)
            .collect(),
    }
}

pub const fn vibe_model_to_dto(value: VibeModel) -> VibeModelDto {
    match value {
        VibeModel::NaiDiffusion45Full => VibeModelDto::NaiDiffusion45Full,
        VibeModel::NaiDiffusion45Curated => VibeModelDto::NaiDiffusion45Curated,
        VibeModel::NaiDiffusion4Full => VibeModelDto::NaiDiffusion4Full,
        VibeModel::NaiDiffusion4Curated => VibeModelDto::NaiDiffusion4Curated,
        VibeModel::NaiDiffusion3 => VibeModelDto::NaiDiffusion3,
        VibeModel::NaiDiffusion3Furry => VibeModelDto::NaiDiffusion3Furry,
    }
}

pub const fn vibe_model_to_domain(value: VibeModelDto) -> VibeModel {
    match value {
        VibeModelDto::NaiDiffusion45Full => VibeModel::NaiDiffusion45Full,
        VibeModelDto::NaiDiffusion45Curated => VibeModel::NaiDiffusion45Curated,
        VibeModelDto::NaiDiffusion4Full => VibeModel::NaiDiffusion4Full,
        VibeModelDto::NaiDiffusion4Curated => VibeModel::NaiDiffusion4Curated,
        VibeModelDto::NaiDiffusion3 => VibeModel::NaiDiffusion3,
        VibeModelDto::NaiDiffusion3Furry => VibeModel::NaiDiffusion3Furry,
    }
}

pub const fn vibe_format_to_domain(value: VibeExportFormatDto) -> VibeExportFormat {
    match value {
        VibeExportFormatDto::Naiv4vibe => VibeExportFormat::Naiv4vibe,
        VibeExportFormatDto::Naiv4vibebundle => VibeExportFormat::Naiv4vibebundle,
    }
}
