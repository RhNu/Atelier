use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactReplayManifest, ArtifactRepository,
    ArtifactResourceReader, ArtifactService, ArtifactSource, RegisterArtifactRequest,
    VisualAssetRef, VisualAssetRole,
};
use atelier_resource_catalog::{
    BlobId, ResourceId, ResourceKind, ResourceLifecycle, ResourceMetadata, ResourceRecord,
    ResourceRef, ResourceState, ResourceVariantKind, VariantId,
};
use futures_executor::block_on;

#[test]
fn registers_generated_artifact_with_primary_resource_ref() {
    block_on(async {
        let repository = FakeArtifactRepository::default();
        let resources = FakeArtifactResourceReader::with_record(
            ResourceId::new("resource-1"),
            ResourceKind::GeneratedImage,
        );
        let service = ArtifactService::new(repository.clone(), resources);

        let record = service
            .register_artifact(generated_request("artifact-1"))
            .await
            .unwrap();

        assert_eq!(record.id, ArtifactId::new("artifact-1"));
        assert_eq!(record.kind, ArtifactKind::GeneratedImage);
        assert_eq!(
            record.primary_resource,
            ResourceRef::base(ResourceId::new("resource-1"))
        );
        assert_eq!(repository.saved(), vec![record]);
    });
}

#[test]
fn rejects_resource_kind_that_does_not_match_artifact_kind() {
    block_on(async {
        let resources = FakeArtifactResourceReader::with_record(
            ResourceId::new("resource-1"),
            ResourceKind::SourceImage,
        );
        let service = ArtifactService::new(FakeArtifactRepository::default(), resources);

        let error = service
            .register_artifact(generated_request("artifact-1"))
            .await
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "invalid_resource_kind: SourceImage cannot back GeneratedImage"
        );
    });
}

#[test]
fn director_artifact_requires_director_resource_kind() {
    block_on(async {
        let resources = FakeArtifactResourceReader::with_record(
            ResourceId::new("resource-1"),
            ResourceKind::DirectorResult,
        );
        let service = ArtifactService::new(FakeArtifactRepository::default(), resources);
        let mut request = generated_request("artifact-1");
        request.kind = ArtifactKind::DirectorResult;
        request.source = ArtifactSource::DirectorRun {
            run_id: "director-1".to_owned(),
        };

        let record = service.register_artifact(request).await.unwrap();

        assert_eq!(record.kind, ArtifactKind::DirectorResult);
    });
}

#[test]
fn preserves_replay_manifest_and_extension_metadata() {
    block_on(async {
        let repository = FakeArtifactRepository::default();
        let resources = FakeArtifactResourceReader::with_record(
            ResourceId::new("resource-1"),
            ResourceKind::StreamFinalImage,
        );
        let service = ArtifactService::new(repository, resources);
        let mut request = generated_request("artifact-1");
        request.metadata.seed = Some(42);
        request.metadata.sample_index = Some(1);
        request
            .metadata
            .extensions
            .insert("novelai:cfg_rescale".to_owned(), "0.25".to_owned());
        request.replay = Some(ArtifactReplayManifest {
            payload_ref: Some("payload:job-1".to_owned()),
            prepared_payload_ref: Some("prepared:job-1".to_owned()),
            prompt_snapshot: Some("1girl, atelier".to_owned()),
            negative_prompt_snapshot: Some("lowres".to_owned()),
        });

        let record = service.register_artifact(request).await.unwrap();

        assert_eq!(record.metadata.seed, Some(42));
        assert_eq!(record.metadata.sample_index, Some(1));
        assert_eq!(
            record.metadata.extensions.get("novelai:cfg_rescale"),
            Some(&"0.25".to_owned())
        );
        assert_eq!(
            record
                .replay
                .as_ref()
                .and_then(|replay| replay.payload_ref.as_deref()),
            Some("payload:job-1")
        );
        assert_eq!(
            record
                .replay
                .as_ref()
                .and_then(|replay| replay.prompt_snapshot.as_deref()),
            Some("1girl, atelier")
        );
    });
}

#[test]
fn preserves_visual_asset_roles_without_physical_paths() {
    block_on(async {
        let resources = FakeArtifactResourceReader::with_record(
            ResourceId::new("resource-1"),
            ResourceKind::GeneratedImage,
        );
        let service = ArtifactService::new(FakeArtifactRepository::default(), resources);
        let mut request = generated_request("artifact-1");
        request.assets.push(VisualAssetRef {
            role: VisualAssetRole::Preview,
            resource: ResourceRef::new(
                ResourceId::new("resource-1"),
                Some(VariantId::new("preview-1")),
            ),
            variant_kind: Some(ResourceVariantKind::Preview),
        });

        let record = service.register_artifact(request).await.unwrap();

        assert_eq!(record.assets.len(), 2);
        assert_eq!(record.assets[0].role, VisualAssetRole::Original);
        assert_eq!(record.assets[1].role, VisualAssetRole::Preview);
        assert_eq!(
            record.assets[1].resource,
            ResourceRef::new(
                ResourceId::new("resource-1"),
                Some(VariantId::new("preview-1"))
            )
        );
    });
}

#[derive(Clone, Default)]
struct FakeArtifactRepository {
    records: Arc<Mutex<Vec<atelier_artifacts::ArtifactRecord>>>,
}

impl FakeArtifactRepository {
    fn saved(&self) -> Vec<atelier_artifacts::ArtifactRecord> {
        self.records.lock().unwrap().clone()
    }
}

#[derive(Clone, Default)]
struct FakeArtifactResourceReader {
    records: Arc<Mutex<BTreeMap<ResourceId, ResourceRecord>>>,
}

impl FakeArtifactResourceReader {
    fn with_record(resource_id: ResourceId, kind: ResourceKind) -> Self {
        let reader = Self::default();
        reader.records.lock().unwrap().insert(
            resource_id.clone(),
            ResourceRecord {
                id: resource_id,
                kind,
                lifecycle: ResourceLifecycle::WorkspaceScoped,
                state: ResourceState::Ready,
                blob_id: BlobId::new("sha256:resource"),
                metadata: ResourceMetadata::default(),
            },
        );
        reader
    }
}

#[async_trait]
impl ArtifactResourceReader for FakeArtifactResourceReader {
    async fn get_artifact_resource(
        &self,
        reference: &ResourceRef,
    ) -> atelier_artifacts::ArtifactResult<ResourceRecord> {
        self.records
            .lock()
            .unwrap()
            .get(&reference.id)
            .cloned()
            .ok_or_else(|| atelier_artifacts::ArtifactError::resource("resource not found"))
    }
}

#[async_trait]
impl ArtifactRepository for FakeArtifactRepository {
    async fn insert_artifact(
        &self,
        record: atelier_artifacts::ArtifactRecord,
    ) -> atelier_artifacts::ArtifactResult<()> {
        self.records.lock().unwrap().push(record);
        Ok(())
    }

    async fn get_artifact(
        &self,
        id: &ArtifactId,
    ) -> atelier_artifacts::ArtifactResult<Option<atelier_artifacts::ArtifactRecord>> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .find(|record| &record.id == id)
            .cloned())
    }

    async fn delete_artifacts(
        &self,
        ids: &[ArtifactId],
    ) -> atelier_artifacts::ArtifactResult<usize> {
        let mut records = self.records.lock().unwrap();
        let original_len = records.len();
        records.retain(|record| !ids.iter().any(|id| id == &record.id));
        Ok(original_len - records.len())
    }
}

fn generated_request(id: &str) -> RegisterArtifactRequest {
    RegisterArtifactRequest {
        id: ArtifactId::new(id),
        kind: ArtifactKind::GeneratedImage,
        source: ArtifactSource::GenerationJob {
            job_id: "job-1".to_owned(),
            batch_id: Some("batch-1".to_owned()),
        },
        primary_resource: ResourceRef::base(ResourceId::new("resource-1")),
        metadata: ArtifactMetadata {
            model_name: Some("nai-diffusion-4-5-full".to_owned()),
            extensions: BTreeMap::new(),
            ..ArtifactMetadata::default()
        },
        replay: None,
        assets: vec![VisualAssetRef {
            role: VisualAssetRole::Original,
            resource: ResourceRef::base(ResourceId::new("resource-1")),
            variant_kind: Some(ResourceVariantKind::Original),
        }],
    }
}
