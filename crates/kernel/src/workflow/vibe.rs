use nai_atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceId, ResourceKind, ResourceLifecycle,
    ResourceOwner, ResourceOwnerKind, ResourceRelation,
};
use nai_atelier_vibe::{
    EncodeVibeRequest, VibeDocumentCodec, VibeDocumentEntry, VibeDocumentResources,
    VibeEncodingRecord, VibeError, VibeExportEntry,
};

use crate::{
    EnsureVibeEncoding, EnsuredVibeEncoding, ExportVibeDocument, ExportedVibeDocument,
    ImportEmbeddedPngVibeDocument, ImportVibeDocument, ImportedVibeDocuments, KernelResult,
    KernelRuntime, KernelVibePorts,
};

pub async fn import_vibe_document<P>(
    runtime: &KernelRuntime<P>,
    request: ImportVibeDocument,
) -> KernelResult<ImportedVibeDocuments>
where
    P: KernelVibePorts,
{
    let imported = VibeDocumentCodec::import_text(&request.file_name, &request.content)?;
    let mut entries = Vec::with_capacity(imported.entries.len());
    for entry in imported.entries {
        let owner = ResourceOwner::new(ResourceOwnerKind::Vibe, entry.summary.document_id.as_str());
        let document = register_vibe_resource(
            runtime,
            ResourceId::new(format!(
                "vibe-document:{}",
                entry.summary.document_id.as_str()
            )),
            ResourceKind::VibeDocument,
            ResourceLifecycle::WorkspaceScoped,
            owner.clone(),
            ResourceRelation::Primary,
            entry.document_payload.as_bytes(),
        )
        .await?;
        let source_image = if let Some(payload) = entry.source_image_payload {
            Some(
                register_vibe_resource(
                    runtime,
                    ResourceId::new(format!(
                        "vibe-source:{}",
                        entry.summary.document_id.as_str()
                    )),
                    ResourceKind::SourceImage,
                    ResourceLifecycle::WorkspaceScoped,
                    owner.clone(),
                    ResourceRelation::Source,
                    payload.as_bytes(),
                )
                .await?,
            )
        } else {
            None
        };
        let preview = if let Some(payload) = entry.preview_payload {
            Some(
                register_vibe_resource(
                    runtime,
                    ResourceId::new(format!(
                        "vibe-preview:{}",
                        entry.summary.document_id.as_str()
                    )),
                    ResourceKind::VibePreview,
                    ResourceLifecycle::WorkspaceScoped,
                    owner.clone(),
                    ResourceRelation::Preview,
                    payload.as_bytes(),
                )
                .await?,
            )
        } else {
            None
        };
        let mut encodings = Vec::with_capacity(entry.encoding_payloads.len());
        for (index, encoding) in entry.encoding_payloads.iter().enumerate() {
            encodings.push(
                register_vibe_resource(
                    runtime,
                    ResourceId::new(format!(
                        "vibe-encoding:{}:{}:{}",
                        entry.summary.document_id.as_str(),
                        encoding.model_key,
                        index
                    )),
                    ResourceKind::VibeEncoding,
                    ResourceLifecycle::WorkspaceScoped,
                    owner.clone(),
                    ResourceRelation::Encoding,
                    encoding.payload.as_bytes(),
                )
                .await?,
            );
        }
        let managed = VibeDocumentEntry {
            summary: entry.summary,
            resources: VibeDocumentResources {
                document,
                source_image,
                preview,
                encodings,
            },
        };
        runtime.ports().insert_document(managed.clone()).await?;
        entries.push(managed);
    }
    Ok(ImportedVibeDocuments { entries })
}

pub async fn import_embedded_png_vibe_document<P>(
    runtime: &KernelRuntime<P>,
    request: ImportEmbeddedPngVibeDocument,
) -> KernelResult<ImportedVibeDocuments>
where
    P: KernelVibePorts,
{
    let Some(content) = runtime
        .ports()
        .extract_embedded_vibe_document_from_png(&request.png_bytes)
        .await?
    else {
        return Err(VibeError::invalid_document(
            "png does not contain an embedded official vibe document",
        )
        .into());
    };
    import_vibe_document(
        runtime,
        ImportVibeDocument {
            file_name: request.file_name,
            content,
        },
    )
    .await
}

pub async fn export_vibe_document<P>(
    runtime: &KernelRuntime<P>,
    request: ExportVibeDocument,
) -> KernelResult<ExportedVibeDocument>
where
    P: KernelVibePorts,
{
    if request.vibe_ids.is_empty() {
        return Err(VibeError::invalid_document("vibe export requires at least one id").into());
    }
    let mut entries = Vec::with_capacity(request.vibe_ids.len());
    for id in request.vibe_ids {
        let entry = runtime.ports().get_document(&id).await?.ok_or_else(|| {
            VibeError::new(nai_atelier_vibe::VibeErrorKind::NotFound, "vibe not found")
        })?;
        let content = runtime
            .ports()
            .read_vibe_document_resource(&entry.resources.document)
            .await?;
        let official_document = serde_json::from_str(&content)
            .map_err(|error| VibeError::invalid_document(error.to_string()))?;
        entries.push(VibeExportEntry { official_document });
    }
    Ok(ExportedVibeDocument {
        document: VibeDocumentCodec::export_text(&entries, request.format)?,
    })
}

pub async fn ensure_vibe_encoding<P>(
    runtime: &KernelRuntime<P>,
    request: EnsureVibeEncoding,
) -> KernelResult<EnsuredVibeEncoding>
where
    P: KernelVibePorts,
{
    if let Some(record) = runtime
        .ports()
        .find_cached_encoding(&request.source, &request.settings)
        .await?
    {
        return Ok(EnsuredVibeEncoding {
            record,
            created: false,
        });
    }

    let encoded = runtime
        .ports()
        .encode_vibe(EncodeVibeRequest {
            image: request.image,
            information_extracted: request.settings.normalized_information_extracted(),
            model: request.settings.model,
            strict_mode: true,
        })
        .await?;
    let resource = runtime
        .ports()
        .register_vibe_resource(RegisterResourceRequest {
            resource_id: ResourceId::new(format!(
                "vibe-encoding-cache:{}:{}:{}:{}",
                request.vibe_id.as_str(),
                &request.source.content_hash,
                request.settings.model.vibe_model_key(),
                request.settings.information_extracted_key()
            )),
            kind: ResourceKind::VibeEncoding,
            lifecycle: ResourceLifecycle::Cache,
            owner: ResourceOwner::new(ResourceOwnerKind::Vibe, request.vibe_id.as_str()),
            relation: ResourceRelation::Encoding,
            blob: BlobWriteIntent::Bytes(encoded.payload.as_bytes().to_vec()),
        })
        .await?;
    let record = VibeEncodingRecord {
        vibe_id: request.vibe_id,
        source: request.source,
        settings: request.settings,
        resource,
    };
    runtime.ports().save_encoding(record.clone()).await?;
    Ok(EnsuredVibeEncoding {
        record,
        created: true,
    })
}

async fn register_vibe_resource<P>(
    runtime: &KernelRuntime<P>,
    resource_id: ResourceId,
    kind: ResourceKind,
    lifecycle: ResourceLifecycle,
    owner: ResourceOwner,
    relation: ResourceRelation,
    bytes: &[u8],
) -> KernelResult<nai_atelier_resource_catalog::ResourceRef>
where
    P: KernelVibePorts,
{
    Ok(runtime
        .ports()
        .register_vibe_resource(RegisterResourceRequest {
            resource_id,
            kind,
            lifecycle,
            owner,
            relation,
            blob: BlobWriteIntent::Bytes(bytes.to_vec()),
        })
        .await?)
}
