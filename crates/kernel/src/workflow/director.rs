use std::collections::BTreeMap;

use nai_atelier_artifacts::{
    ArtifactId, ArtifactKind, ArtifactMetadata, ArtifactSource, RegisterArtifactRequest,
    VisualAssetRef, VisualAssetRole,
};
use nai_atelier_resource_catalog::{
    BlobWriteIntent, RegisterResourceRequest, ResourceId, ResourceKind, ResourceLifecycle,
    ResourceOwner, ResourceOwnerKind, ResourceRelation, ResourceVariantKind,
};

use crate::{
    KernelClock, KernelDirectorPorts, KernelEventKind, KernelEventSink, KernelResult,
    KernelRuntime, RanDirectorTool, RunDirectorTool,
};

pub async fn run_director_tool<P>(
    runtime: &mut KernelRuntime<P>,
    request: RunDirectorTool,
) -> KernelResult<RanDirectorTool>
where
    P: KernelClock + KernelDirectorPorts + KernelEventSink,
{
    let run_id = request.run_id;
    let tool_request = request.request.normalize_for_tool()?;
    let output = runtime.ports().run_director_tool(tool_request).await?;
    let resource = runtime
        .ports()
        .register_director_resource(RegisterResourceRequest {
            resource_id: ResourceId::new(format!("resource:director:{run_id}")),
            kind: ResourceKind::DirectorResult,
            lifecycle: ResourceLifecycle::WorkspaceScoped,
            owner: ResourceOwner::new(ResourceOwnerKind::DirectorRun, run_id.clone()),
            relation: ResourceRelation::Primary,
            blob: BlobWriteIntent::Bytes(output.bytes),
        })
        .await?;
    let artifact_id = ArtifactId::new(format!("director:{run_id}"));
    let artifact = runtime
        .ports()
        .register_director_artifact(RegisterArtifactRequest {
            id: artifact_id.clone(),
            kind: ArtifactKind::DirectorResult,
            source: ArtifactSource::DirectorRun {
                run_id: run_id.clone(),
            },
            primary_resource: resource.clone(),
            metadata: ArtifactMetadata {
                seed: output.seed,
                sample_index: None,
                model_name: None,
                extensions: BTreeMap::default(),
            },
            replay: None,
            assets: vec![VisualAssetRef {
                role: VisualAssetRole::Original,
                resource: resource.clone(),
                variant_kind: Some(ResourceVariantKind::Original),
            }],
        })
        .await?;
    let safety = match runtime.ports().score_director_image(resource.clone()).await {
        Ok(assessment) => assessment,
        Err(error) => {
            runtime
                .emit(KernelEventKind::DirectorSafetyScanFailed {
                    run_id,
                    resource: resource.clone(),
                    message: error.to_string(),
                })
                .await;
            None
        }
    };
    let item = runtime
        .ports()
        .index_director_gallery_item(artifact, runtime.ports().now_ms(), safety)
        .await?;
    Ok(RanDirectorTool {
        resource,
        artifact_id,
        item,
    })
}
