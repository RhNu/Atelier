use async_trait::async_trait;
use atelier_resource_catalog::{ResourceRecord, ResourceRef};

use crate::{ArtifactId, ArtifactRecord, ArtifactResult};

#[async_trait]
pub trait ArtifactRepository: Send + Sync {
    async fn insert_artifact(&self, record: ArtifactRecord) -> ArtifactResult<()>;

    async fn delete_artifacts(&self, ids: &[ArtifactId]) -> ArtifactResult<usize>;
}

#[async_trait]
pub trait ArtifactResourceReader: Send + Sync {
    async fn get_artifact_resource(
        &self,
        reference: &ResourceRef,
    ) -> ArtifactResult<ResourceRecord>;
}
