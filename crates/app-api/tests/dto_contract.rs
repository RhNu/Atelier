use nai_atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
    ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto,
};
use nai_atelier_app_api::error::ErrorEnvelopeDto;
use nai_atelier_app_api::event::{AppEventKindDto, AppEventPageDto, EventsSinceRequestDto};
use nai_atelier_app_api::gallery::{
    GalleryImageReferenceRequestDto, GalleryImageReferenceTargetDto,
};
use nai_atelier_app_api::generation::{
    GenerationStatusQueryDto, ImageModelDto, QueueDirectiveDto, RunGenerationJobRequestDto,
};
use nai_atelier_app_api::prompt::{
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, GetPromptChunkRequestDto,
    ListPromptChunksRequestDto, PromptChunkPageDto, PromptLexiconSearchQueryDto,
};
use nai_atelier_app_api::settings::{
    GenerationDefaultsDto, ImageVariantSettingsDto, ResetWorkspaceSettingsResponseDto,
    UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
};
use nai_atelier_app_api::workspace::CloseWorkspaceResponseDto;
use serde_json::json;

#[test]
fn serializes_novelai_model_wire_names() {
    let text = serde_json::to_string(&ImageModelDto::NaiDiffusion45Full).unwrap();

    assert_eq!(text, "\"nai-diffusion-4-5-full\"");
}

#[test]
fn queue_directive_uses_stable_tagged_shape() {
    let value = serde_json::to_value(QueueDirectiveDto::StartJob {
        job_id: "job-1".to_owned(),
    })
    .unwrap();

    assert_eq!(value, json!({ "kind": "start_job", "job_id": "job-1" }));
}

#[test]
fn error_envelope_keeps_code_message_and_json_details() {
    let error = ErrorEnvelopeDto {
        code: "invalid_request".to_owned(),
        message: "bad input".to_owned(),
        details: Some(json!({ "field": "prompt" })),
    };

    assert_eq!(
        serde_json::to_value(error).unwrap(),
        json!({
            "code": "invalid_request",
            "message": "bad input",
            "details": { "field": "prompt" }
        })
    );
}

#[test]
fn api_key_responses_and_debug_never_expose_secret_values() {
    let record = ApiKeyRecordDto {
        id: "main".to_owned(),
        display_name: "Main".to_owned(),
        is_active: true,
    };
    let request = CreateApiKeyRequestDto {
        id: "main".to_owned(),
        display_name: "Main".to_owned(),
        secret: "nai-secret-token".to_owned(),
    };

    let response_text = serde_json::to_string(&record).unwrap();
    let debug_text = format!("{request:?}");

    assert!(!response_text.contains("secret"));
    assert!(!debug_text.contains("nai-secret-token"));
    assert!(debug_text.contains("<redacted>"));
}

#[test]
fn account_command_requests_use_plain_ids_without_secrets() {
    assert_eq!(
        serde_json::to_value(DeleteApiKeyRequestDto {
            id: "main".to_owned()
        })
        .unwrap(),
        json!({ "id": "main" })
    );
    assert_eq!(
        serde_json::to_value(SetActiveApiKeyRequestDto {
            id: "main".to_owned()
        })
        .unwrap(),
        json!({ "id": "main" })
    );
    assert_eq!(
        serde_json::to_value(ProbeApiKeyRequestDto {
            id: "main".to_owned()
        })
        .unwrap(),
        json!({ "id": "main" })
    );
    assert_eq!(
        serde_json::to_value(DeleteApiKeyResponseDto { deleted: true }).unwrap(),
        json!({ "deleted": true })
    );
}

#[test]
fn prompt_command_dtos_have_stable_page_and_delete_shapes() {
    assert_eq!(
        serde_json::to_value(GetPromptChunkRequestDto {
            chunk_id: Some("chunk-1".to_owned()),
            key: None,
        })
        .unwrap(),
        json!({ "chunk_id": "chunk-1" })
    );
    assert_eq!(
        serde_json::to_value(ListPromptChunksRequestDto {
            offset: 5,
            limit: 10,
        })
        .unwrap(),
        json!({ "offset": 5, "limit": 10 })
    );
    assert_eq!(
        serde_json::to_value(DeletePromptChunkRequestDto {
            chunk_id: "chunk-1".to_owned()
        })
        .unwrap(),
        json!({ "chunk_id": "chunk-1" })
    );
    assert_eq!(
        serde_json::to_value(DeletePromptChunkResponseDto { deleted: false }).unwrap(),
        json!({ "deleted": false })
    );
    assert_eq!(
        serde_json::to_value(PromptChunkPageDto {
            items: Vec::new(),
            total: 0,
            offset: 5,
            limit: 10,
        })
        .unwrap(),
        json!({ "items": [], "total": 0, "offset": 5, "limit": 10 })
    );
    assert_eq!(
        serde_json::to_value(PromptLexiconSearchQueryDto {
            query: "1girl".to_owned(),
            limit: 20,
        })
        .unwrap(),
        json!({ "query": "1girl", "limit": 20 })
    );
}

#[test]
fn generation_workspace_and_event_command_dtos_are_command_friendly() {
    assert_eq!(
        serde_json::to_value(RunGenerationJobRequestDto {
            job_id: "job-1".to_owned()
        })
        .unwrap(),
        json!({ "job_id": "job-1" })
    );
    assert_eq!(
        serde_json::to_value(GenerationStatusQueryDto {
            job_id: Some("job-1".to_owned())
        })
        .unwrap(),
        json!({ "job_id": "job-1" })
    );
    assert_eq!(
        serde_json::to_value(CloseWorkspaceResponseDto { was_open: true }).unwrap(),
        json!({ "was_open": true })
    );
    assert_eq!(
        serde_json::to_value(EventsSinceRequestDto {
            sequence: 7,
            limit: 50,
        })
        .unwrap(),
        json!({ "sequence": 7, "limit": 50 })
    );
    assert_eq!(
        serde_json::to_value(AppEventPageDto {
            items: vec![nai_atelier_app_api::event::AppEventDto {
                sequence: 8,
                kind: AppEventKindDto::JobSucceeded {
                    batch_id: "batch-1".to_owned(),
                    job_id: "job-1".to_owned(),
                },
            }],
            next_sequence: 8,
        })
        .unwrap(),
        json!({
            "items": [{
                "sequence": 8,
                "kind": {
                    "kind": "job_succeeded",
                    "batch_id": "batch-1",
                    "job_id": "job-1"
                }
            }],
            "next_sequence": 8
        })
    );
}

#[test]
fn gallery_image_reference_target_uses_stable_wire_names() {
    assert_eq!(
        serde_json::to_value(GalleryImageReferenceRequestDto {
            item_id: "artifact:job-1:sample:0".to_owned(),
            target: GalleryImageReferenceTargetDto::PreciseReference,
        })
        .unwrap(),
        json!({
            "item_id": "artifact:job-1:sample:0",
            "target": "precise_reference"
        })
    );
}

#[test]
fn workspace_settings_dtos_have_stable_json_field_names() {
    let settings = WorkspaceSettingsDto {
        generation: GenerationDefaultsDto::default(),
        image_variants: ImageVariantSettingsDto {
            thumbnail_long_edge: 320,
            preview_long_edge: 1024,
        },
    };

    assert_eq!(
        serde_json::to_value(UpdateWorkspaceSettingsRequestDto {
            settings: settings.clone(),
        })
        .unwrap(),
        json!({
            "settings": {
                "generation": {
                    "model": "nai-diffusion-4-5-full",
                    "size": { "width": 832, "height": 1216 },
                    "quality": true,
                    "uc_preset": "light",
                    "steps": 23,
                    "scale": 5.0,
                    "sampler": "k_euler_ancestral",
                    "noise_schedule": "karras",
                    "seed": 0,
                    "n_samples": 1,
                    "cfg_rescale": 0.0,
                    "variety_boost": false,
                    "strict_mode": false
                },
                "image_variants": {
                    "thumbnail_long_edge": 320,
                    "preview_long_edge": 1024
                }
            }
        })
    );
    assert_eq!(
        serde_json::to_value(ResetWorkspaceSettingsResponseDto { settings }).unwrap(),
        json!({
            "settings": {
                "generation": {
                    "model": "nai-diffusion-4-5-full",
                    "size": { "width": 832, "height": 1216 },
                    "quality": true,
                    "uc_preset": "light",
                    "steps": 23,
                    "scale": 5.0,
                    "sampler": "k_euler_ancestral",
                    "noise_schedule": "karras",
                    "seed": 0,
                    "n_samples": 1,
                    "cfg_rescale": 0.0,
                    "variety_boost": false,
                    "strict_mode": false
                },
                "image_variants": {
                    "thumbnail_long_edge": 320,
                    "preview_long_edge": 1024
                }
            }
        })
    );
}
