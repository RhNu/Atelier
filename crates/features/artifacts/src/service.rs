use crate::{
    ArtifactError, ArtifactId, ArtifactRecord, ArtifactRepository, ArtifactResourceReader,
    ArtifactResult, RegisterArtifactRequest,
};

#[derive(Clone, Debug)]
pub struct ArtifactService<R, C> {
    repository: R,
    resources: C,
}

impl<R, C> ArtifactService<R, C> {
    #[must_use]
    pub const fn new(repository: R, resources: C) -> Self {
        Self {
            repository,
            resources,
        }
    }
}

impl<R, C> ArtifactService<R, C>
where
    R: ArtifactRepository,
    C: ArtifactResourceReader,
{
    /// Registers an artifact after validating its primary catalog resource kind.
    ///
    /// # Errors
    /// Returns an error when the resource kind cannot back the artifact kind or
    /// when repository persistence fails.
    pub async fn register_artifact(
        &self,
        request: RegisterArtifactRequest,
    ) -> ArtifactResult<ArtifactRecord> {
        let primary_record = self
            .resources
            .get_artifact_resource(&request.primary_resource)
            .await?;
        if !request.kind.accepts_resource_kind(primary_record.kind) {
            return Err(ArtifactError::invalid_resource_kind(format!(
                "{:?} cannot back {:?}",
                primary_record.kind, request.kind
            )));
        }

        let record = ArtifactRecord {
            id: request.id,
            kind: request.kind,
            source: request.source,
            primary_resource: request.primary_resource,
            metadata: request.metadata,
            replay: request.replay,
            assets: request.assets,
        };
        self.repository.insert_artifact(record.clone()).await?;
        Ok(record)
    }

    /// Deletes artifact records by id.
    ///
    /// # Errors
    /// Returns an error when repository persistence fails.
    pub async fn delete_artifacts(&self, ids: &[ArtifactId]) -> ArtifactResult<usize> {
        self.repository.delete_artifacts(ids).await
    }
}
