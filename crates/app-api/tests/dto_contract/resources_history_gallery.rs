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
        serde_json::to_value(ReleaseImportedImageResourcesRequestDto {
            resources: vec![ResourceRefDto {
                id: "resource:import:source:1".to_owned(),
                variant_id: None,
            }],
        })
        .unwrap(),
        json!({ "resources": [{ "id": "resource:import:source:1" }] })
    );
    assert_eq!(
        serde_json::to_value(ReleaseImportedImageResourcesResponseDto {
            released: 1,
            resources_deleted: 1,
            blobs_deleted: 1,
        })
        .unwrap(),
        json!({ "released": 1, "resources_deleted": 1, "blobs_deleted": 1 })
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
        serde_json::to_value(SaveResourceImageRequestDto {
            resource: ResourceRefDto {
                id: "resource:image:1".to_owned(),
                variant_id: None,
            },
            format: Some(ImageExportFormatDto::PngSanitized),
            suggested_file_name: Some("job-1-sample-0".to_owned()),
        })
        .unwrap(),
        json!({
            "resource": { "id": "resource:image:1" },
            "format": "png_sanitized",
            "suggested_file_name": "job-1-sample-0"
        })
    );
    assert_eq!(
        serde_json::to_value(CopyResourceImageRequestDto {
            resource: ResourceRefDto {
                id: "resource:image:1".to_owned(),
                variant_id: None,
            },
            format: ImageExportFormatDto::Jpeg,
        })
        .unwrap(),
        json!({
            "resource": { "id": "resource:image:1" },
            "format": "jpeg"
        })
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
fn vibe_catalog_dtos_have_stable_metadata_shapes() {
    let entry = VibeDocumentEntryDto {
        vibe_id: "vibe-1".to_owned(),
        display_name: "Style A".to_owned(),
        has_image: true,
        hidden: false,
        available_model_keys: vec!["v4-5full".to_owned()],
        available_encoding_configs: vec![VibeEncodingConfigDto {
            model: VibeModelDto::NaiDiffusion45Full,
            information_extracted: 0.7,
        }],
        created_at_ms: 10,
        updated_at_ms: 20,
        document: ResourceRefDto {
            id: "vibe-document:vibe-1".to_owned(),
            variant_id: None,
        },
        source_image: Some(ResourceRefDto {
            id: "vibe-source:vibe-1".to_owned(),
            variant_id: None,
        }),
        preview: Some(ResourceRefDto {
            id: "vibe-preview:vibe-1".to_owned(),
            variant_id: None,
        }),
        encodings: vec![ResourceRefDto {
            id: "vibe-encoding:vibe-1:v4-5full:0".to_owned(),
            variant_id: None,
        }],
    };

    assert_eq!(
        serde_json::to_value(ListVibeDocumentsRequestDto {
            offset: 0,
            limit: 20,
            include_hidden: true,
        })
        .unwrap(),
        json!({ "offset": 0, "limit": 20, "include_hidden": true })
    );
    assert_eq!(
        serde_json::to_value(GetVibeDocumentRequestDto {
            vibe_id: "vibe-1".to_owned()
        })
        .unwrap(),
        json!({ "vibe_id": "vibe-1" })
    );
    assert_eq!(
        serde_json::to_value(VibeDocumentPageDto {
            items: vec![entry],
            total: 1,
            offset: 0,
            limit: 20,
        })
        .unwrap(),
        json!({
            "items": [{
                "vibe_id": "vibe-1",
                "display_name": "Style A",
                "has_image": true,
                "hidden": false,
                "available_model_keys": ["v4-5full"],
                "available_encoding_configs": [{
                    "model": "nai-diffusion-4-5-full",
                    "information_extracted": 0.7_f32
                }],
                "created_at_ms": 10,
                "updated_at_ms": 20,
                "document": { "id": "vibe-document:vibe-1" },
                "source_image": { "id": "vibe-source:vibe-1" },
                "preview": { "id": "vibe-preview:vibe-1" },
                "encodings": [{ "id": "vibe-encoding:vibe-1:v4-5full:0" }]
            }],
            "total": 1,
            "offset": 0,
            "limit": 20
        })
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
                    "sample_index": 0,
                    "artifact_id": "artifact:job-1:sample:0",
                    "item_id": "gallery:job-1:sample:0",
                    "resource": {
                        "id": "resource:job-1:sample:0",
                        "variant_id": "preview"
                    },
                    "asset_role": "preview",
                    "variant_kind": "preview",
                    "state": "available"
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
                    "sample_index": 0,
                    "artifact_id": "artifact:job-1:sample:0",
                    "item_id": "gallery:job-1:sample:0",
                    "resource": {
                        "id": "resource:job-1:sample:0",
                        "variant_id": "preview"
                    },
                    "asset_role": "preview",
                    "variant_kind": "preview",
                    "state": "available"
                }]
            }
        })
    );
}

#[test]
fn delete_run_history_dtos_have_stable_shapes() {
    assert_eq!(
        serde_json::to_value(DeleteRunHistoryItemsRequestDto {
            run_ids: vec!["job-1".to_owned(), "job-2".to_owned()],
        })
        .unwrap(),
        json!({ "run_ids": ["job-1", "job-2"] })
    );
    assert_eq!(
        serde_json::to_value(DeleteRunHistoryItemsResponseDto { deleted: 2 }).unwrap(),
        json!({ "deleted": 2 })
    );
}

#[test]
fn generation_batch_history_dtos_have_stable_shapes() {
    let batch = GenerationHistoryBatchDto {
        batch_id: "batch-1".to_owned(),
        status: GenerationBatchHistoryStatusDto::PartiallySucceeded,
        title: Some("1girl".to_owned()),
        last_error: Some("request 2 failed".to_owned()),
        created_at_ms: 10,
        updated_at_ms: 20,
        completed_at_ms: Some(20),
        request_count: 2,
        completed_request_count: 2,
        expected_sample_count: 3,
        completed_sample_count: 2,
        available_sample_count: 2,
        outputs: Vec::new(),
    };
    assert_eq!(
        serde_json::to_value(GenerationHistoryPageDto {
            items: vec![batch],
            total: 1,
            offset: 0,
            limit: 8,
        })
        .unwrap(),
        json!({
            "items": [{
                "batch_id": "batch-1",
                "status": "partially_succeeded",
                "title": "1girl",
                "last_error": "request 2 failed",
                "created_at_ms": 10,
                "updated_at_ms": 20,
                "completed_at_ms": 20,
                "request_count": 2,
                "completed_request_count": 2,
                "expected_sample_count": 3,
                "completed_sample_count": 2,
                "available_sample_count": 2,
                "outputs": []
            }],
            "total": 1,
            "offset": 0,
            "limit": 8
        })
    );
    assert_eq!(
        serde_json::to_value(GenerationHistoryQueryDto {
            offset: 8,
            limit: 8,
            status: Some(GenerationBatchHistoryStatusDto::PartiallySucceeded),
        })
        .unwrap(),
        json!({ "offset": 8, "limit": 8, "status": "partially_succeeded" })
    );
}

#[test]
fn generation_batch_commands_status_and_zip_dtos_have_stable_shapes() {
    assert_eq!(
        serde_json::to_value(RerunGenerationHistoryBatchRequestDto {
            source_batch_id: "batch-1".to_owned(),
            batch_id: "batch-2".to_owned(),
            job_ids: vec!["job-3".to_owned(), "job-4".to_owned()],
        })
        .unwrap(),
        json!({
            "source_batch_id": "batch-1",
            "batch_id": "batch-2",
            "job_ids": ["job-3", "job-4"]
        })
    );
    assert_eq!(
        serde_json::to_value(DeleteGenerationHistoryBatchesRequestDto {
            batch_ids: vec!["batch-1".to_owned()],
        })
        .unwrap(),
        json!({ "batch_ids": ["batch-1"] })
    );
    assert_eq!(
        serde_json::to_value(DeleteGenerationHistoryBatchesResponseDto {
            deleted_requests: 2,
        })
        .unwrap(),
        json!({ "deleted_requests": 2 })
    );
    assert_eq!(
        serde_json::to_value(SaveResourceImagesZipRequestDto {
            entries: vec![SaveResourceImagesZipEntryDto {
                resource: ResourceRefDto {
                    id: "resource:sample".to_owned(),
                    variant_id: None,
                },
                file_name: "request-01_sample-01".to_owned(),
            }],
            suggested_file_name: Some("batch-1".to_owned()),
        })
        .unwrap(),
        json!({
            "entries": [{
                "resource": { "id": "resource:sample" },
                "file_name": "request-01_sample-01"
            }],
            "suggested_file_name": "batch-1"
        })
    );
    assert_eq!(
        serde_json::to_value(GenerationStatusDto {
            batch_id: Some("batch-1".to_owned()),
            batch_status: Some("running".to_owned()),
            current_job_id: Some("job-2".to_owned()),
            job_status: Some("running".to_owned()),
            requests: vec![GenerationRequestStatusDto {
                job_id: "job-2".to_owned(),
                request_index: 1,
                expected_samples: 4,
                status: "running".to_owned(),
            }],
        })
        .unwrap(),
        json!({
            "batch_id": "batch-1",
            "batch_status": "running",
            "current_job_id": "job-2",
            "job_status": "running",
            "requests": [{
                "job_id": "job-2",
                "request_index": 1,
                "expected_samples": 4,
                "status": "running"
            }]
        })
    );
}

#[test]
fn delete_gallery_item_dtos_have_stable_shapes() {
    assert_eq!(
        serde_json::to_value(DeleteGalleryItemsRequestDto {
            item_ids: vec!["artifact:job-1:sample:0".to_owned()],
        })
        .unwrap(),
        json!({ "item_ids": ["artifact:job-1:sample:0"] })
    );
    assert_eq!(
        serde_json::to_value(DeleteGalleryItemsResponseDto {
            deleted: 1,
            resources_released: 1,
            blobs_deleted: 5,
        })
        .unwrap(),
        json!({
            "deleted": 1,
            "resources_released": 1,
            "blobs_deleted": 5
        })
    );
}
