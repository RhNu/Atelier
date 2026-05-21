use async_trait::async_trait;
use atelier_kernel::KernelVibePorts;
use atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceCatalogError, ResourceRef, ResourceResult,
};
use atelier_vibe::{
    EmbeddedVibeDocumentExtractor, EncodeVibeRequest, EncodedVibe, NovelAiVibeClient,
    VibeDocumentEntry, VibeDomainResult, VibeEncodeSettings, VibeEncodingRecord, VibeError,
    VibeErrorKind, VibeId, VibeRepository, VibeResult, VibeSourceIdentity,
};

use super::{MemoryKernelPorts, RegisteredResource};

#[async_trait]
impl NovelAiVibeClient for MemoryKernelPorts {
    async fn encode_vibe(&self, request: EncodeVibeRequest) -> VibeResult<EncodedVibe> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("encode_vibe".to_owned());
        if state.encoded_vibe_payload.is_empty() {
            Ok(EncodedVibe {
                payload: format!(
                    "encoded:{}:{}",
                    request.model.vibe_model_key(),
                    request.information_extracted
                ),
            })
        } else {
            Ok(EncodedVibe {
                payload: state.encoded_vibe_payload.clone(),
            })
        }
    }
}

#[async_trait]
impl EmbeddedVibeDocumentExtractor for MemoryKernelPorts {
    async fn extract_embedded_vibe_document_from_png(
        &self,
        _png_bytes: &[u8],
    ) -> VibeDomainResult<Option<String>> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("extract_embedded_vibe".to_owned());
        Ok(state.embedded_vibe_document.clone())
    }
}

#[async_trait]
impl VibeRepository for MemoryKernelPorts {
    async fn insert_document(&self, entry: VibeDocumentEntry) -> VibeDomainResult<VibeId> {
        let id = entry.document_id().clone();
        self.state
            .lock()
            .unwrap()
            .vibe_documents
            .insert(id.as_str().to_owned(), entry);
        Ok(id)
    }

    async fn get_document(&self, id: &VibeId) -> VibeDomainResult<Option<VibeDocumentEntry>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .vibe_documents
            .get(id.as_str())
            .cloned())
    }

    async fn find_cached_encoding(
        &self,
        source: &VibeSourceIdentity,
        settings: &VibeEncodeSettings,
    ) -> VibeDomainResult<Option<VibeEncodingRecord>> {
        self.state
            .lock()
            .unwrap()
            .operations
            .push("find_vibe_encoding".to_owned());
        Ok(self
            .state
            .lock()
            .unwrap()
            .vibe_cache
            .get(&settings.cache_key(source))
            .cloned())
    }

    async fn save_encoding(&self, record: VibeEncodingRecord) -> VibeDomainResult<()> {
        let key = record.settings.cache_key(&record.source);
        let mut state = self.state.lock().unwrap();
        state.operations.push("save_vibe_encoding".to_owned());
        state.vibe_cache.insert(key, record);
        Ok(())
    }
}

#[async_trait]
impl KernelVibePorts for MemoryKernelPorts {
    async fn register_vibe_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        let mut state = self.state.lock().unwrap();
        state
            .operations
            .push(format!("register_vibe_resource:{:?}", request.kind));
        if state.failures.contains(&super::FakeFailure::Resource) {
            return Err(ResourceCatalogError::repository("resource failed"));
        }
        let BlobWriteIntent::Bytes(bytes) = request.blob;
        let resource_id = request.resource_id;
        state.resources.insert(
            resource_id.as_str().to_owned(),
            RegisteredResource {
                kind: request.kind,
                bytes,
            },
        );
        Ok(ResourceRef::base(resource_id))
    }

    async fn read_vibe_document_resource(
        &self,
        reference: &ResourceRef,
    ) -> VibeDomainResult<String> {
        let bytes = self
            .state
            .lock()
            .unwrap()
            .resources
            .get(reference.id.as_str())
            .map(|resource| resource.bytes.clone())
            .ok_or_else(|| VibeError::new(VibeErrorKind::NotFound, "missing vibe document"))?;
        String::from_utf8(bytes).map_err(|error| VibeError::invalid_document(error.to_string()))
    }
}
