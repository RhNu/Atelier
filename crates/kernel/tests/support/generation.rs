use async_trait::async_trait;
use atelier_artifacts::{ArtifactError, ArtifactRecord, ArtifactResult, RegisterArtifactRequest};
use atelier_gallery::{GalleryItem, GalleryResult};
use atelier_generation::{
    GenerateImageRequest, GenerateImageResult, GenerateImageStreamRequest,
    GenerateImageStreamResult, GeneratedImageMetadata, GeneratedImageMetadataInspector,
    GenerationResult, NovelAiGenerationClient,
};
use atelier_image_analysis::{ImageAnalysisModelId, ImageAnalysisModelInfo, ImageRatingScores};
use atelier_kernel::{
    GenerationPayloadStore, KernelGenerationPorts, PreparedGenerationPayload,
    SubmittedGenerationPayload,
};
use atelier_prompt_resources::{
    CompilePromptRequest, CompiledPrompt, PromptResourceResult, PromptTrace,
};
use atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceCatalogError, ResourceRef, ResourceResult,
};
use atelier_safety::{
    ImageSafetyScore, SafetyAssessment, SafetyError, SafetyLabel, SafetyModelEvidence,
    SafetyResult, SafetyReviewOutcome, SafetyRiskBand,
};
use futures_util::stream;

use super::{FakeFailure, MemoryKernelPorts, RegisteredResource};

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
        state.compiled_models.push(request.model);
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
        let fused_score = ImageSafetyScore::new(0.1).unwrap();
        Ok(Some(SafetyAssessment {
            resource,
            auto_label: SafetyLabel::Safe,
            risk_band: SafetyRiskBand::Low,
            policy_id: "test-policy".to_owned(),
            policy_version: "1".to_owned(),
            primary: SafetyModelEvidence {
                model: ImageAnalysisModelInfo {
                    id: ImageAnalysisModelId::AnimeDbRating,
                    revision: "test".to_owned(),
                },
                ratings: ImageRatingScores::new(0.9, 0.0, 0.1, 0.0).unwrap(),
                fused_score,
            },
            review: SafetyReviewOutcome::NotNeeded,
            assessed_at_ms: None,
        }))
    }

    async fn index_gallery_item(
        &self,
        artifact: ArtifactRecord,
        indexed_at_ms: u64,
        safety: atelier_gallery::GallerySafetyState,
    ) -> GalleryResult<GalleryItem> {
        let mut state = self.state.lock().unwrap();
        state.operations.push("index_gallery".to_owned());
        if state.failures.contains(&FakeFailure::Gallery) {
            return Err(atelier_gallery::GalleryError::repository("gallery failed"));
        }
        let item = GalleryItem::from_artifact(artifact, indexed_at_ms, safety);
        state
            .gallery_items
            .insert(item.id.as_str().to_owned(), item.clone());
        Ok(item)
    }
}

#[async_trait]
impl NovelAiGenerationClient for MemoryKernelPorts {
    async fn generate(
        &self,
        request: GenerateImageRequest,
    ) -> GenerationResult<GenerateImageResult> {
        let mut state = self.state.lock().unwrap();
        let resolved_seed = if request.seed == 0 {
            4242
        } else {
            request.seed
        };
        state.operations.push("generate".to_owned());
        state.generated_requests.push(request);
        if let Some(error) = state.fail_generate.clone() {
            return Err(error);
        }
        Ok(GenerateImageResult {
            resolved_seed,
            images: state.generated_images.clone(),
        })
    }

    async fn generate_stream(
        &self,
        request: GenerateImageStreamRequest,
    ) -> GenerationResult<GenerateImageStreamResult> {
        let mut state = self.state.lock().unwrap();
        let resolved_seed = if request.base.seed == 0 {
            4242
        } else {
            request.base.seed
        };
        state.operations.push("generate_stream".to_owned());
        state.stream_requests.push(request);
        let items = state.stream_items.drain(..).collect::<Vec<_>>();
        Ok(GenerateImageStreamResult {
            resolved_seed,
            stream: Box::pin(stream::iter(items)),
        })
    }
}

impl GeneratedImageMetadataInspector for MemoryKernelPorts {
    fn inspect_generated_image_metadata(
        &self,
        _bytes: &[u8],
        _mime_type: Option<&str>,
    ) -> GeneratedImageMetadata {
        GeneratedImageMetadata::UnsupportedFormat
    }
}
