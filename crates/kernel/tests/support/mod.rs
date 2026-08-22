#![allow(clippy::significant_drop_tightening)]
#![allow(dead_code)]

mod artifact_gallery;
mod director;
mod generation;
mod precise_reference;
mod vibe;

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_artifacts::ArtifactRecord;
use atelier_director::DirectorToolOutput;
use atelier_gallery::GalleryItem;
use atelier_generation::{
    GenerateImageRequest, GenerateImageStreamRequest, GeneratedImage, GenerationClientError,
    GenerationResult, ImageModel, ImageStreamEvent,
};
use atelier_kernel::{
    KernelClock, KernelEvent, KernelEventSink, PreparedGenerationPayload,
    SubmittedGenerationPayload,
};
use atelier_precise_reference::PreciseReferenceImage;
use atelier_resource_catalog::{ResourceKind, ResourceRef};
use atelier_vibe::{VibeDocumentEntry, VibeEncodingRecord};

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
    compiled_models: Vec<ImageModel>,
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

    /// Models every `compile_prompt` call was made against, in call order.
    pub fn compiled_models(&self) -> Vec<ImageModel> {
        self.state.lock().unwrap().compiled_models.clone()
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
