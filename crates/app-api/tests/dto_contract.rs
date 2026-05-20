use nai_atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
    ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto,
};
use nai_atelier_app_api::director::{
    DirectorToolDto, DirectorToolResultDto, RunDirectorToolRequestDto,
};
use nai_atelier_app_api::error::ErrorEnvelopeDto;
use nai_atelier_app_api::event::{AppEventKindDto, AppEventPageDto, EventsSinceRequestDto};
use nai_atelier_app_api::gallery::{
    GalleryImageReferenceRequestDto, GalleryImageReferenceTargetDto, GalleryItemDto,
    GallerySafetyDto, GallerySafetyLabelDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
    GallerySourceKindDto,
};
use nai_atelier_app_api::generation::{
    CharacterDto, CharacterPositionDto, CharacterReferenceDto, CharacterReferenceTypeDto,
    ControlNetConfigDto, ControlNetInputDto, GenerateImageRequestDto, GenerationStatusQueryDto,
    ImageModelDto, Img2ImgRequestDto, QueueDirectiveDto, RunGenerationJobRequestDto,
};
use nai_atelier_app_api::prompt::{
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, GetPromptChunkRequestDto,
    ListPromptChunksRequestDto, PromptChunkPageDto, PromptLexiconSearchQueryDto,
};
use nai_atelier_app_api::resource::{
    ImageInputDto, ImageResourceKindDto, ImportImageResourceRequestDto,
    ImportImageResourceResponseDto, ResourceRefDto,
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
    assert_eq!(
        serde_json::to_value(AppEventKindDto::DirectorSafetyScanFailed {
            run_id: "director-1".to_owned(),
            resource: ResourceRefDto {
                id: "resource:director:director-1".to_owned(),
                variant_id: None,
            },
            message: "scanner unavailable".to_owned(),
        })
        .unwrap(),
        json!({
            "kind": "director_safety_scan_failed",
            "run_id": "director-1",
            "resource": {
                "id": "resource:director:director-1"
            },
            "message": "scanner unavailable"
        })
    );
}

#[test]
fn generation_request_exposes_resource_backed_drawing_inputs() {
    let request = GenerateImageRequestDto {
        prompt: "1girl".to_owned(),
        i2i: Some(Img2ImgRequestDto {
            image: ImageInputDto::resource("source-image"),
            strength: 0.45,
            noise: 0.2,
            mask: Some(ImageInputDto::resource("mask-image")),
        }),
        controlnet: Some(ControlNetConfigDto {
            images: vec![ControlNetInputDto {
                vibe_data_cache: "cache-key".to_owned(),
                info_extracted: 0.7,
                strength: 0.8,
            }],
            strength: 0.5,
        }),
        character_references: Some(vec![CharacterReferenceDto {
            image: ImageInputDto::resource("character-ref"),
            reference_type: CharacterReferenceTypeDto::CharacterAndStyle,
            fidelity: 0.6,
            strength: 0.75,
        }]),
        characters: Some(vec![CharacterDto {
            prompt: "hero".to_owned(),
            negative_prompt: Some("lowres".to_owned()),
            position: CharacterPositionDto { x: 0.25, y: 0.75 },
            enabled: true,
        }]),
        use_coords: Some(true),
        ..GenerateImageRequestDto::default()
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({
            "prompt": "1girl",
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
            "strict_mode": false,
            "i2i": {
                "image": {
                    "kind": "resource_ref",
                    "resource": { "id": "source-image" }
                },
                "strength": 0.45_f32,
                "noise": 0.2_f32,
                "mask": {
                    "kind": "resource_ref",
                    "resource": { "id": "mask-image" }
                }
            },
            "controlnet": {
                "images": [{
                    "vibe_data_cache": "cache-key",
                    "info_extracted": 0.7_f32,
                    "strength": 0.8_f32
                }],
                "strength": 0.5_f32
            },
            "character_references": [{
                "image": {
                    "kind": "resource_ref",
                    "resource": { "id": "character-ref" }
                },
                "reference_type": "character_and_style",
                "fidelity": 0.6_f32,
                "strength": 0.75_f32
            }],
            "characters": [{
                "prompt": "hero",
                "negative_prompt": "lowres",
                "position": { "x": 0.25_f32, "y": 0.75_f32 },
                "enabled": true
            }],
            "use_coords": true
        })
    );
}

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
}

#[test]
fn director_command_dtos_use_resource_inputs_and_gallery_results() {
    let result = DirectorToolResultDto {
        item_id: "artifact:director:run-1".to_owned(),
        artifact_id: "director:run-1".to_owned(),
        resource: ResourceRefDto {
            id: "resource:director:run-1".to_owned(),
            variant_id: None,
        },
        item: GalleryItemDto {
            item_id: "artifact:director:run-1".to_owned(),
            artifact_id: "director:run-1".to_owned(),
            artifact_kind: "director_result".to_owned(),
            source_kind: GallerySourceKindDto::Director,
            primary_resource: ResourceRefDto {
                id: "resource:director:run-1".to_owned(),
                variant_id: None,
            },
            assets: Vec::new(),
            indexed_at_ms: 123,
            seed: None,
            sample_index: None,
            model_name: None,
            safety: None,
            manual_safety_override: None,
        },
    };

    assert_eq!(
        serde_json::to_value(RunDirectorToolRequestDto {
            run_id: "run-1".to_owned(),
            tool: DirectorToolDto::Lineart,
            image: ImageInputDto::resource("source-image"),
            prompt: Some("clean lineart".to_owned()),
            defry: Some(2),
            strict_mode: true,
        })
        .unwrap(),
        json!({
            "run_id": "run-1",
            "tool": "lineart",
            "image": {
                "kind": "resource_ref",
                "resource": { "id": "source-image" }
            },
            "prompt": "clean lineart",
            "defry": 2,
            "strict_mode": true
        })
    );
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!({
            "item_id": "artifact:director:run-1",
            "artifact_id": "director:run-1",
            "resource": { "id": "resource:director:run-1" },
            "item": {
                "item_id": "artifact:director:run-1",
                "artifact_id": "director:run-1",
                "artifact_kind": "director_result",
                "source_kind": "director",
                "primary_resource": { "id": "resource:director:run-1" },
                "assets": [],
                "indexed_at_ms": 123
            }
        })
    );
}

#[test]
fn gallery_item_can_report_complete_safety_assessment() {
    let safety = GallerySafetyDto {
        scan_state: GallerySafetyScanStateDto::Scanned,
        risk_band: Some(GallerySafetyRiskBandDto::High),
        auto_label: Some(GallerySafetyLabelDto::Sensitive),
        effective_label: GallerySafetyLabelDto::Sensitive,
        nsfw_score: Some(0.91),
        safe_score: Some(0.09),
        raw_scores: vec![
            nai_atelier_app_api::gallery::GallerySafetyScoreDto {
                label: "safe".to_owned(),
                score: 0.09,
            },
            nai_atelier_app_api::gallery::GallerySafetyScoreDto {
                label: "nsfw".to_owned(),
                score: 0.91,
            },
        ],
        model_id: Some("open_nsfw@onnx".to_owned()),
        scorer_version: Some("1".to_owned()),
        assessed_at_ms: Some(123),
    };

    assert_eq!(
        serde_json::to_value(safety).unwrap(),
        json!({
            "scan_state": "scanned",
            "risk_band": "high",
            "auto_label": "sensitive",
            "effective_label": "sensitive",
            "nsfw_score": 0.91_f32,
            "safe_score": 0.09_f32,
            "raw_scores": [
                { "label": "safe", "score": 0.09_f32 },
                { "label": "nsfw", "score": 0.91_f32 }
            ],
            "model_id": "open_nsfw@onnx",
            "scorer_version": "1",
            "assessed_at_ms": 123
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
