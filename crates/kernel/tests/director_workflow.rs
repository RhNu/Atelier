mod support;

use atelier_director::{DirectorTool, RunDirectorToolRequest};
use atelier_kernel::{KernelEventKind, KernelRuntime, RunDirectorTool};
use atelier_resource_catalog::ResourceKind;
use futures_executor::block_on;

use support::MemoryKernelPorts;

#[test]
fn director_tool_result_is_persisted_and_indexed() {
    block_on(async {
        let ports = MemoryKernelPorts::default().with_director_output(vec![4, 5, 6], Some(7));
        let mut runtime = KernelRuntime::new(ports.clone());

        let result = runtime
            .run_director_tool(RunDirectorTool {
                run_id: "director-1".to_owned(),
                request: RunDirectorToolRequest {
                    tool: DirectorTool::Lineart,
                    image: "AQID".to_owned(),
                    prompt: Some("clean lines".to_owned()),
                    defry: Some(2),
                    strict_mode: true,
                },
            })
            .await
            .unwrap();

        assert_eq!(result.resource.id.as_str(), "resource:director:director-1");
        assert_eq!(result.artifact_id.as_str(), "director:director-1");
        assert_eq!(
            result.item.artifact_kind,
            atelier_artifacts::ArtifactKind::DirectorResult
        );
        assert_eq!(
            ports.operations(),
            vec![
                "run_director_tool",
                "register_resource:DirectorResult",
                "register_artifact",
                "score_image",
                "index_gallery"
            ]
        );
        assert_eq!(
            ports.registered_resources()["resource:director:director-1"].kind,
            ResourceKind::DirectorResult
        );
        let artifact = &ports.artifacts()["director:director-1"];
        assert_eq!(artifact.metadata.seed, Some(7));
        assert_eq!(artifact.metadata.sample_index, None);
        assert!(artifact.replay.is_none());
        assert!(
            ports
                .gallery_items()
                .contains_key("artifact:director:director-1")
        );
    });
}

#[test]
fn director_safety_failure_is_reported_without_blocking_gallery_indexing() {
    block_on(async {
        let ports = MemoryKernelPorts::default()
            .with_director_output(vec![4, 5, 6], Some(7))
            .failing_safety();
        let mut runtime = KernelRuntime::new(ports.clone());

        let result = runtime
            .run_director_tool(RunDirectorTool {
                run_id: "director-1".to_owned(),
                request: RunDirectorToolRequest {
                    tool: DirectorTool::Lineart,
                    image: "AQID".to_owned(),
                    prompt: None,
                    defry: None,
                    strict_mode: true,
                },
            })
            .await
            .unwrap();

        assert!(result.item.safety_assessment.is_none());
        assert!(ports.gallery_items().contains_key(result.item.id.as_str()));
        assert!(ports.events().iter().any(|event| {
            matches!(
                &event.kind,
                KernelEventKind::DirectorSafetyScanFailed { run_id, .. }
                    if run_id == "director-1"
            )
        }));
    });
}
