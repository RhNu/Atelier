use super::*;

#[test]
fn image_resource_import_dtos_are_resource_catalog_oriented() {
    assert_eq!(
        serde_json::to_value(ImportImageResourceRequestDto {
            kind: ImageResourceKindDto::ReferenceImage,
            image_base64: "AQID".to_owned(),
            mime_type: Some("image/png".to_owned()),
        })
        .unwrap(),
        json!({
            "kind": "reference_image",
            "image_base64": "AQID",
            "mime_type": "image/png"
        })
    );
    assert_eq!(
        serde_json::to_value(ImportImageResourceResponseDto {
            resource: ResourceRefDto {
                id: "resource:image:1".to_owned(),
                variant_id: None,
            },
        })
        .unwrap(),
        json!({ "resource": { "id": "resource:image:1" } })
    );
    assert_eq!(
        serde_json::to_value(GetResourceImageRequestDto {
            resource: ResourceRefDto {
                id: "resource:image:1".to_owned(),
                variant_id: Some("preview".to_owned()),
            },
        })
        .unwrap(),
        json!({ "resource": { "id": "resource:image:1", "variant_id": "preview" } })
    );
    assert_eq!(
        serde_json::to_value(ResourceImageDto {
            image_base64: "AQID".to_owned(),
            mime_type: Some("image/png".to_owned()),
        })
        .unwrap(),
        json!({ "image_base64": "AQID", "mime_type": "image/png" })
    );
}

#[test]
fn run_history_query_and_page_dtos_have_stable_shapes() {
    assert_eq!(
        serde_json::to_value(RunHistoryQueryDto {
            offset: 5,
            limit: 25,
            kind: Some(RunHistoryKindDto::Generation),
            status: Some(RunHistoryStatusDto::Paused),
        })
        .unwrap(),
        json!({
            "offset": 5,
            "limit": 25,
            "kind": "generation",
            "status": "paused"
        })
    );
    assert_eq!(
        serde_json::to_value(RunHistoryPageDto {
            items: vec![sample_run_history_item()],
            total: 1,
            offset: 0,
            limit: 50,
        })
        .unwrap(),
        json!({
            "items": [{
                "run_id": "job-1",
                "kind": "generation",
                "status": "succeeded",
                "batch_id": "batch-1",
                "job_id": "job-1",
                "origin_run_id": "job-0",
                "title": "1girl",
                "created_at_ms": 10,
                "updated_at_ms": 20,
                "completed_at_ms": 20,
                "recoverable": false,
                "outputs": [{
                    "artifact_id": "artifact:job-1:sample:0",
                    "item_id": "gallery:job-1:sample:0",
                    "resource": {
                        "id": "resource:job-1:sample:0",
                        "variant_id": "preview"
                    },
                    "asset_role": "preview",
                    "variant_kind": "preview"
                }]
            }],
            "total": 1,
            "offset": 0,
            "limit": 50
        })
    );
}

#[test]
fn rerun_generation_history_dtos_have_stable_shapes() {
    assert_eq!(
        serde_json::to_value(RerunGenerationHistoryItemRequestDto {
            run_id: "job-1".to_owned(),
            batch_id: "batch-2".to_owned(),
            job_id: "job-2".to_owned(),
        })
        .unwrap(),
        json!({
            "run_id": "job-1",
            "batch_id": "batch-2",
            "job_id": "job-2"
        })
    );
    assert_eq!(
        serde_json::to_value(RerunGenerationHistoryItemResponseDto {
            directive: QueueDirectiveDto::StartJob {
                job_id: "job-2".to_owned(),
            },
            item: sample_run_history_item(),
        })
        .unwrap(),
        json!({
            "directive": {
                "kind": "start_job",
                "job_id": "job-2"
            },
            "item": {
                "run_id": "job-1",
                "kind": "generation",
                "status": "succeeded",
                "batch_id": "batch-1",
                "job_id": "job-1",
                "origin_run_id": "job-0",
                "title": "1girl",
                "created_at_ms": 10,
                "updated_at_ms": 20,
                "completed_at_ms": 20,
                "recoverable": false,
                "outputs": [{
                    "artifact_id": "artifact:job-1:sample:0",
                    "item_id": "gallery:job-1:sample:0",
                    "resource": {
                        "id": "resource:job-1:sample:0",
                        "variant_id": "preview"
                    },
                    "asset_role": "preview",
                    "variant_kind": "preview"
                }]
            }
        })
    );
}
