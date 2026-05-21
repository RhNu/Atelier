use super::*;

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
    let update = UpdateApiKeyRequestDto {
        id: "main".to_owned(),
        display_name: Some("Renamed".to_owned()),
        secret: Some("nai-secret-token".to_owned()),
    };

    let response_text = serde_json::to_string(&record).unwrap();
    let debug_text = format!("{request:?}");
    let update_debug_text = format!("{update:?}");

    assert!(!response_text.contains("secret"));
    assert!(!response_text.contains("nai-secret-token"));
    assert!(!debug_text.contains("nai-secret-token"));
    assert!(!update_debug_text.contains("nai-secret-token"));
    assert!(debug_text.contains("<redacted>"));
    assert!(update_debug_text.contains("<redacted>"));
}

#[test]
fn api_key_create_update_requests_accept_secret_only_as_input_payload() {
    assert_eq!(
        serde_json::to_value(CreateApiKeyRequestDto {
            id: "main".to_owned(),
            display_name: "Main".to_owned(),
            secret: "nai-secret-token".to_owned(),
        })
        .unwrap(),
        json!({
            "id": "main",
            "display_name": "Main",
            "secret": "nai-secret-token"
        })
    );
    assert_eq!(
        serde_json::to_value(UpdateApiKeyRequestDto {
            id: "main".to_owned(),
            display_name: None,
            secret: Some("nai-secret-token".to_owned()),
        })
        .unwrap(),
        json!({
            "id": "main",
            "secret": "nai-secret-token"
        })
    );
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
            items: vec![atelier_app_api::event::AppEventDto {
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
