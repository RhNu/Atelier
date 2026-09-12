use atelier_artifacts::ArtifactSource;
use atelier_gallery::{GalleryItem, GalleryResult};
use atelier_jobs::{RunHistoryRepository, RunOutputRecord, RunOutputState};

use super::{AppKernelPorts, gallery_repository_error};
use crate::mapping::{resource_variant_kind_as_str, visual_asset_role_as_str};

impl<S: Send + Sync, F: Send + Sync, E: Send + Sync> AppKernelPorts<S, F, E> {
    pub(super) async fn record_outputs(&self, item: &GalleryItem) -> GalleryResult<()> {
        let run_id = match &item.source {
            ArtifactSource::GenerationJob { job_id, .. } => job_id,
            ArtifactSource::DirectorRun { run_id } => run_id,
        };
        for asset in &item.assets {
            self.run_history
                .upsert_run_output(RunOutputRecord {
                    run_id: run_id.clone(),
                    sample_index: item.metadata.sample_index,
                    artifact_id: item.artifact_id.as_str().to_owned(),
                    item_id: Some(item.id.as_str().to_owned()),
                    resource_id: Some(asset.resource.id.as_str().to_owned()),
                    variant_id: asset
                        .resource
                        .variant_id
                        .as_ref()
                        .map(|id| id.as_str().to_owned()),
                    asset_role: visual_asset_role_as_str(asset.role).to_owned(),
                    variant_kind: asset
                        .variant_kind
                        .map(resource_variant_kind_as_str)
                        .map(str::to_owned),
                    state: RunOutputState::Available,
                })
                .await
                .map_err(gallery_repository_error)?;
        }
        Ok(())
    }
}
