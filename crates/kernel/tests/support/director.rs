use async_trait::async_trait;
use atelier_artifacts::{ArtifactRecord, ArtifactResult, RegisterArtifactRequest};
use atelier_director::{
    DirectorResult, DirectorToolOutput, NovelAiDirectorClient, RunDirectorToolRequest,
};
use atelier_gallery::{GalleryItem, GalleryResult, GallerySafetyState};
use atelier_kernel::{KernelDirectorPorts, KernelGenerationPorts};
use atelier_resource_catalog::{RegisterResourceRequest, ResourceRef, ResourceResult};
use atelier_safety::{SafetyAssessment, SafetyResult};

use super::MemoryKernelPorts;

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
        safety: GallerySafetyState,
    ) -> GalleryResult<GalleryItem> {
        KernelGenerationPorts::index_gallery_item(self, artifact, indexed_at_ms, safety).await
    }
}
