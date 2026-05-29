#![allow(clippy::significant_drop_tightening)]
#![allow(dead_code)]

mod vibe;

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_artifacts::{
    ArtifactError, ArtifactRecord, ArtifactRepository, ArtifactResourceReader, ArtifactResult,
    RegisterArtifactRequest,
};
use atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use atelier_gallery::{GalleryIndex, GalleryItem, GalleryItemId, GalleryResult};
use atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationClientError,
    GenerationResult, ImageStreamEvent, ImageStreamResult, NovelAiGenerationClient,
};
use atelier_kernel::{
    GenerationPayloadStore, KernelClock, KernelDirectorPorts, KernelEvent, KernelEventSink,
    KernelGenerationPorts, KernelPreciseReferencePorts, PreparedGenerationPayload,
    SubmittedGenerationPayload,
};
use atelier_precise_reference::{PreciseReferenceImage, PreciseReferenceResult};
use atelier_prompt_resources::{
    CompilePromptRequest, CompiledPrompt, PromptResourceResult, PromptTrace,
};
use atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceCatalogError, ResourceKind, ResourceMetadata,
    ResourceRecord, ResourceRef, ResourceResult, ResourceState,
};
use atelier_safety::{ImageSafetyScore, SafetyAssessment, SafetyError, SafetyResult};
use atelier_vibe::{VibeDocumentEntry, VibeEncodingRecord};
use futures_util::stream;

#[derive(Clone, Default)]
pub struct MemoryKernelPorts {
    state: Arc<Mutex<State>>,
}

#[derive(Default)]
struct State {
    submitted: BTreeMap<String, SubmittedGenerationPayload>,
    prepared: BTreeMap<String, PreparedGenerationPayload>,
    events: Vec<KernelEvent>,
    operations: Vec<String>,
    expanded_prompt: String,
    compiled_prompts: BTreeMap<String, String>,
    generated_images: Vec<GeneratedImage>,
    generated_requests: Vec<GenerateImageRequest>,
    stream_requests: Vec<GenerateImageStreamRequest>,
    director_output: Option<DirectorToolOutput>,
    stream_items: VecDeque<GenerationResult<ImageStreamEvent>>,
    resources: BTreeMap<String, RegisteredResource>,
    precise_reference_images: BTreeMap<String, PreciseReferenceImage>,
    artifacts: BTreeMap<String, ArtifactRecord>,
    gallery_items: BTreeMap<String, GalleryItem>,
    vibe_cache: BTreeMap<String, VibeEncodingRecord>,
    vibe_documents: BTreeMap<String, VibeDocumentEntry>,
    embedded_vibe_document: Option<String>,
    encoded_vibe_payload: String,
    fail_generate: Option<GenerationClientError>,
    failures: HashSet<FakeFailure>,
    now_ms: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum FakeFailure {
    CompilePrompt,
    PreparedPayload,
    Resource,
    Artifact,
    Gallery,
    Safety,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredResource {
    pub kind: ResourceKind,
    pub bytes: Vec<u8>,
}

impl MemoryKernelPorts {
    pub fn with_expanded_prompt(self, prompt: &str) -> Self {
        let mut state = self.state.lock().unwrap();
        prompt.clone_into(&mut state.expanded_prompt);
        drop(state);
        self
    }

    pub fn with_compiled_prompt(self, raw: &str, expanded: &str) -> Self {
        self.state
            .lock()
            .unwrap()
            .compiled_prompts
            .insert(raw.to_owned(), expanded.to_owned());
        self
    }

    pub fn with_generated_images(self, images: Vec<GeneratedImage>) -> Self {
        self.state.lock().unwrap().generated_images = images;
        self
    }

    pub fn with_director_output(self, bytes: Vec<u8>, seed: Option<i64>) -> Self {
        self.state.lock().unwrap().director_output = Some(DirectorToolOutput {
            bytes,
            mime_type: Some("image/png".to_owned()),
            seed,
        });
        self
    }

    pub fn with_stream_items(self, items: Vec<GenerationResult<ImageStreamEvent>>) -> Self {
        self.state.lock().unwrap().stream_items = VecDeque::from(items);
        self
    }

    pub fn with_encoded_vibe_payload(self, payload: &str) -> Self {
        payload.clone_into(&mut self.state.lock().unwrap().encoded_vibe_payload);
        self
    }

    pub fn with_embedded_vibe_document(self, document: &str) -> Self {
        self.state.lock().unwrap().embedded_vibe_document = Some(document.to_owned());
        self
    }

    pub fn with_cached_vibe_encoding(self, record: VibeEncodingRecord) -> Self {
        let key = record.settings.cache_key(&record.source);
        self.state.lock().unwrap().vibe_cache.insert(key, record);
        self
    }

    pub fn with_precise_reference_image(
        self,
        reference: &ResourceRef,
        kind: ResourceKind,
        payload: &str,
    ) -> Self {
        self.state.lock().unwrap().precise_reference_images.insert(
            reference.id.as_str().to_owned(),
            PreciseReferenceImage {
                kind,
                payload: payload.to_owned(),
            },
        );
        self
    }

    pub fn failing_generate(self, error: GenerationClientError) -> Self {
        self.state.lock().unwrap().fail_generate = Some(error);
        self
    }

    pub fn failing_compile_prompt(self) -> Self {
        self.failing(FakeFailure::CompilePrompt)
    }

    pub fn failing_prepared_payload(self) -> Self {
        self.failing(FakeFailure::PreparedPayload)
    }

    pub fn failing_resource(self) -> Self {
        self.failing(FakeFailure::Resource)
    }

    pub fn failing_artifact(self) -> Self {
        self.failing(FakeFailure::Artifact)
    }

    pub fn failing_gallery(self) -> Self {
        self.failing(FakeFailure::Gallery)
    }

    pub fn failing_safety(self) -> Self {
        self.failing(FakeFailure::Safety)
    }

    fn failing(self, failure: FakeFailure) -> Self {
        self.state.lock().unwrap().failures.insert(failure);
        self
    }

    pub fn operations(&self) -> Vec<String> {
        self.state.lock().unwrap().operations.clone()
    }

    pub fn events(&self) -> Vec<KernelEvent> {
        self.state.lock().unwrap().events.clone()
    }

    pub fn submitted_payload_count(&self) -> usize {
        self.state.lock().unwrap().submitted.len()
    }

    pub fn submitted_prompt(&self, payload_ref: &str) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .submitted
            .get(payload_ref)
            .map(|payload| payload.request.prompt().to_owned())
    }

    pub fn compile_call_count(&self) -> usize {
        self.operations()
            .into_iter()
            .filter(|operation| operation == "compile_prompt")
            .count()
    }

    pub fn generate_call_count(&self) -> usize {
        self.operations()
            .into_iter()
            .filter(|operation| operation == "generate")
            .count()
    }

    pub fn generated_requests(&self) -> Vec<GenerateImageRequest> {
        self.state.lock().unwrap().generated_requests.clone()
    }

    pub fn stream_requests(&self) -> Vec<GenerateImageStreamRequest> {
        self.state.lock().unwrap().stream_requests.clone()
    }

    pub fn encode_vibe_call_count(&self) -> usize {
        self.operations()
            .into_iter()
            .filter(|operation| operation == "encode_vibe")
            .count()
    }

    pub fn registered_resources(&self) -> BTreeMap<String, RegisteredResource> {
        self.state.lock().unwrap().resources.clone()
    }

    pub fn artifacts(&self) -> BTreeMap<String, ArtifactRecord> {
        self.state.lock().unwrap().artifacts.clone()
    }

    pub fn gallery_items(&self) -> BTreeMap<String, GalleryItem> {
        self.state.lock().unwrap().gallery_items.clone()
    }
}

#[async_trait]
impl KernelEventSink for MemoryKernelPorts {
    async fn emit(&self, event: KernelEvent) {
        self.state.lock().unwrap().events.push(event);
    }
}

impl KernelClock for MemoryKernelPorts {
    fn now_ms(&self) -> u64 {
        self.state.lock().unwrap().now_ms
    }
}

#[async_trait]
impl GenerationPayloadStore for MemoryKernelPorts {
    async fn save_submitted_payload(
        &self,
        payload: SubmittedGenerationPayload,
    ) -> atelier_kernel::KernelResult<()> {
        let key = payload.payload_ref.as_str().to_owned();
        let mut state = self.state.lock().unwrap();
        state.operations.push("save_submitted".to_owned());
        state.submitted.insert(key, payload);
        Ok(())
    }

    async fn save_submitted_payloads(
        &self,
        payloads: Vec<SubmittedGenerationPayload>,
    ) -> atelier_kernel::KernelResult<()> {
        let mut state = self.state.lock().unwrap();
        for payload in payloads {
            let key = payload.payload_ref.as_str().to_owned();
            state.operations.push("save_submitted".to_owned());
            state.submitted.insert(key, payload);
        }
        Ok(())
    }

    async fn get_submitted_payload(
        &self,
        payload_ref: &atelier_jobs::JobPayloadRef,
    ) -> atelier_kernel::KernelResult<Option<SubmittedGenerationPayload>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .submitted
            .get(payload_ref.as_str())
            .cloned())
    }

    async fn save_prepared_payload(
        &self,
        payload: PreparedGenerationPayload,
    ) -> atelier_kernel::KernelResult<()> {
        let key = payload.payload_ref.as_str().to_owned();
        let mut state = self.state.lock().unwrap();
        state.operations.push("save_prepared".to_owned());
        if state.failures.contains(&FakeFailure::PreparedPayload) {
            return Err(atelier_kernel::KernelError::PayloadStore(
                "prepared payload failed".to_owned(),
            ));
        }
        state.prepared.insert(key, payload);
        Ok(())
    }
}

#[async_trait]
impl KernelGenerationPorts for MemoryKernelPorts {
    async fn compile_prompt(
        &self,
        request: CompilePromptRequest,
    ) -> PromptResourceResult<CompiledPrompt> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("compile_prompt".to_owned());
        if state.failures.contains(&FakeFailure::CompilePrompt) {
            return Err(atelier_prompt_resources::PromptResourceError::repository(
                "compile failed",
            ));
        }
        let expanded = state
            .compiled_prompts
            .get(&request.prompt)
            .cloned()
            .or_else(|| (!state.expanded_prompt.is_empty()).then(|| state.expanded_prompt.clone()))
            .unwrap_or_else(|| request.prompt.clone());
        Ok(CompiledPrompt {
            expanded_prompt: expanded.clone(),
            trace: PromptTrace {
                raw_prompt: request.prompt,
                expanded_prompt: expanded,
                function_calls: Vec::new(),
            },
        })
    }

    async fn register_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        let mut state = self.state.lock().unwrap();
        state
            .operations
            .push(format!("register_resource:{:?}", request.kind));
        if state.failures.contains(&FakeFailure::Resource) {
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

    async fn register_artifact(
        &self,
        request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord> {
        let record = ArtifactRecord {
            id: request.id,
            kind: request.kind,
            source: request.source,
            primary_resource: request.primary_resource,
            metadata: request.metadata,
            replay: request.replay,
            assets: request.assets,
        };
        let mut state = self.state.lock().unwrap();
        state.operations.push("register_artifact".to_owned());
        if state.failures.contains(&FakeFailure::Artifact) {
            return Err(ArtifactError::repository("artifact failed"));
        }
        state
            .artifacts
            .insert(record.id.as_str().to_owned(), record.clone());
        Ok(record)
    }

    async fn score_image(&self, resource: ResourceRef) -> SafetyResult<Option<SafetyAssessment>> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("score_image".to_owned());
        if state.failures.contains(&FakeFailure::Safety) {
            return Err(SafetyError::scanner("scanner unavailable"));
        }
        Ok(Some(SafetyAssessment::new(
            resource,
            ImageSafetyScore::new(0.1).unwrap(),
        )))
    }

    async fn index_gallery_item(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety_assessment: Option<SafetyAssessment>,
    ) -> GalleryResult<GalleryItem> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("index_gallery".to_owned());
        if state.failures.contains(&FakeFailure::Gallery) {
            return Err(atelier_gallery::GalleryError::repository("gallery failed"));
        }
        let item = GalleryItem::from_artifact(artifact, indexed_at_ms, safety_assessment);
        state
            .gallery_items
            .insert(item.id.as_str().to_owned(), item.clone());
        Ok(item)
    }
}

#[async_trait]
impl NovelAiDirectorClient for MemoryKernelPorts {
    async fn run_director_tool(
        &self,
        _request: RunDirectorToolRequest,
    ) -> DirectorResult<DirectorToolOutput> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("run_director_tool".to_owned());
        Ok(state
            .director_output
            .clone()
            .unwrap_or_else(|| DirectorToolOutput {
                bytes: vec![4, 5, 6],
                mime_type: Some("image/png".to_owned()),
                seed: None,
            }))
    }
}

#[async_trait]
impl KernelDirectorPorts for MemoryKernelPorts {
    async fn register_director_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> ResourceResult<ResourceRef> {
        KernelGenerationPorts::register_resource(self, request).await
    }

    async fn register_director_artifact(
        &self,
        request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord> {
        KernelGenerationPorts::register_artifact(self, request).await
    }

    async fn score_director_image(
        &self,
        resource: ResourceRef,
    ) -> SafetyResult<Option<SafetyAssessment>> {
        KernelGenerationPorts::score_image(self, resource).await
    }

    async fn index_director_gallery_item(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety_assessment: Option<SafetyAssessment>,
    ) -> GalleryResult<GalleryItem> {
        KernelGenerationPorts::index_gallery_item(self, artifact, indexed_at_ms, safety_assessment)
            .await
    }
}

#[async_trait]
impl NovelAiGenerationClient for MemoryKernelPorts {
    async fn generate(
        &self,
        request: atelier_generation::GenerateImageRequest,
    ) -> GenerationResult<Vec<GeneratedImage>> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("generate".to_owned());
        state.generated_requests.push(request);
        if let Some(error) = state.fail_generate.clone() {
            return Err(error);
        }
        Ok(state.generated_images.clone())
    }

    async fn generate_stream(
        &self,
        request: atelier_generation::GenerateImageStreamRequest,
    ) -> GenerationResult<ImageStreamResult> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("generate_stream".to_owned());
        state.stream_requests.push(request);
        let items = state.stream_items.drain(..).collect::<Vec<_>>();
        Ok(Box::pin(stream::iter(items)))
    }
}

#[async_trait]
impl KernelPreciseReferencePorts for MemoryKernelPorts {
    async fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> PreciseReferenceResult<PreciseReferenceImage> {
        self.state
            .lock()
            .unwrap()
            .precise_reference_images
            .get(source.id.as_str())
            .cloned()
            .ok_or_else(|| {
                atelier_precise_reference::PreciseReferenceError::not_found(
                    "precise reference image is missing",
                )
            })
    }
}

#[async_trait]
impl ArtifactRepository for MemoryKernelPorts {
    async fn insert_artifact(&self, record: ArtifactRecord) -> ArtifactResult<()> {
        self.state
            .lock()
            .unwrap()
            .artifacts
            .insert(record.id.as_str().to_owned(), record);
        Ok(())
    }
}

#[async_trait]
impl ArtifactResourceReader for MemoryKernelPorts {
    async fn get_artifact_resource(
        &self,
        reference: &ResourceRef,
    ) -> ArtifactResult<ResourceRecord> {
        Ok(ResourceRecord {
            id: reference.id.clone(),
            kind: self.state.lock().unwrap().resources[reference.id.as_str()].kind,
            lifecycle: atelier_resource_catalog::ResourceLifecycle::JobScoped,
            state: ResourceState::Ready,
            blob_id: atelier_resource_catalog::BlobId::new("blob"),
            metadata: ResourceMetadata::default(),
        })
    }
}

#[async_trait]
impl GalleryIndex for MemoryKernelPorts {
    async fn upsert_item(&self, item: GalleryItem) -> GalleryResult<()> {
        self.state
            .lock()
            .unwrap()
            .gallery_items
            .insert(item.id.as_str().to_owned(), item);
        Ok(())
    }

    async fn get_item(&self, id: &GalleryItemId) -> GalleryResult<Option<GalleryItem>> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .gallery_items
            .get(id.as_str())
            .cloned())
    }

    async fn query_items(
        &self,
        query: atelier_gallery::GalleryQuery,
    ) -> GalleryResult<Vec<GalleryItem>> {
        Ok(query.apply(self.state.lock().unwrap().gallery_items.values().cloned()))
    }

    async fn set_safety_override(
        &self,
        id: &GalleryItemId,
        manual_safety_override: Option<atelier_gallery::GallerySafetyOverride>,
    ) -> GalleryResult<GalleryItem> {
        let mut state = self.state.lock().unwrap();
        let item = state
            .gallery_items
            .get_mut(id.as_str())
            .ok_or_else(|| atelier_gallery::GalleryError::not_found("missing item"))?;
        item.manual_safety_override = manual_safety_override;
        Ok(item.clone())
    }
}
