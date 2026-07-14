use atelier_app_api::account::{
    ApiKeyRecordDto, CreateApiKeyRequestDto, DeleteApiKeyRequestDto, DeleteApiKeyResponseDto,
    ProbeApiKeyRequestDto, SetActiveApiKeyRequestDto, UpdateApiKeyRequestDto,
};
use atelier_app_api::director::{
    DirectorToolDto, DirectorToolResultDto, RunDirectorToolRequestDto,
};
use atelier_app_api::error::ErrorEnvelopeDto;
use atelier_app_api::event::{AppEventKindDto, AppEventPageDto, EventsSinceRequestDto};
use atelier_app_api::gallery::{
    DeleteGalleryItemsRequestDto, DeleteGalleryItemsResponseDto, GalleryImageReferenceRequestDto,
    GalleryImageReferenceTargetDto, GalleryItemDto, GallerySafetyDto, GallerySafetyLabelDto,
    GallerySafetyRiskBandDto, GallerySafetyScanStateDto, GallerySourceKindDto,
};
use atelier_app_api::generation::{
    CharacterDto, CharacterPositionDto, CharacterReferenceDto, CharacterReferenceTypeDto,
    ControlNetConfigDto, ControlNetInputDto, GenerateImageRequestDto, GenerationAnlasEstimateDto,
    GenerationEstimateRequestDto, GenerationStatusQueryDto, ImageModelDto, Img2ImgRequestDto,
    QueueDirectiveDto, RunGenerationJobRequestDto, StreamModeDto, SubmitGenerationBatchJobDto,
    SubmitGenerationBatchRequestDto,
};
use atelier_app_api::history::{
    DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
    RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto, RunHistoryItemDto,
    RunHistoryKindDto, RunHistoryOutputDto, RunHistoryPageDto, RunHistoryQueryDto,
    RunHistoryStatusDto,
};
use atelier_app_api::prompt::{
    CompileGenerationCharacterPromptDto, CompileGenerationPromptRequestDto,
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, GetPromptChunkRequestDto,
    ListPromptChunksRequestDto, ListPromptPresetsRequestDto, PromptChunkPageDto,
    PromptLexiconSearchQueryDto, PromptPresetKindDto, UpsertPromptPresetRequestDto,
};
use atelier_app_api::resource::{
    GetResourceImageRequestDto, ImageInputDto, ImageResourceKindDto, ImportImageResourceRequestDto,
    ImportImageResourceResponseDto, ReleaseImportedImageResourcesRequestDto,
    ReleaseImportedImageResourcesResponseDto, ResourceImageDto, ResourceRefDto,
    SaveResourceImageRequestDto,
};
use atelier_app_api::settings::{
    FrontendGallerySettingsDto, FrontendSettingsDto, GenerationDefaultsDto,
    ImageVariantSettingsDto, ResetWorkspaceSettingsResponseDto, UpdateWorkspaceSettingsRequestDto,
    WorkspaceSettingsDto,
};
use atelier_app_api::vibe::{
    GetVibeDocumentRequestDto, ListVibeDocumentsRequestDto, VibeDocumentEntryDto,
    VibeDocumentPageDto, VibeEncodingConfigDto, VibeModelDto,
};
use atelier_app_api::workspace::CloseWorkspaceResponseDto;
use serde_json::json;

#[path = "dto_contract/account_prompt_generation.rs"]
mod account_prompt_generation;
#[path = "dto_contract/resources_history_gallery.rs"]
mod resources_history_gallery;

fn sample_run_history_item() -> RunHistoryItemDto {
    RunHistoryItemDto {
        run_id: "job-1".to_owned(),
        kind: RunHistoryKindDto::Generation,
        status: RunHistoryStatusDto::Succeeded,
        batch_id: Some("batch-1".to_owned()),
        job_id: Some("job-1".to_owned()),
        origin_run_id: Some("job-0".to_owned()),
        title: Some("1girl".to_owned()),
        last_error: None,
        created_at_ms: 10,
        updated_at_ms: 20,
        completed_at_ms: Some(20),
        recoverable: false,
        outputs: vec![RunHistoryOutputDto {
            artifact_id: "artifact:job-1:sample:0".to_owned(),
            item_id: Some("gallery:job-1:sample:0".to_owned()),
            resource: ResourceRefDto {
                id: "resource:job-1:sample:0".to_owned(),
                variant_id: Some("preview".to_owned()),
            },
            asset_role: "preview".to_owned(),
            variant_kind: Some("preview".to_owned()),
        }],
    }
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
            atelier_app_api::gallery::GallerySafetyScoreDto {
                label: "safe".to_owned(),
                score: 0.09,
            },
            atelier_app_api::gallery::GallerySafetyScoreDto {
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
        frontend: FrontendSettingsDto {
            gallery: FrontendGallerySettingsDto {
                blur_sensitive_images: true,
            },
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
                },
                "frontend": {
                    "gallery": {
                        "blur_sensitive_images": true
                    }
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
                },
                "frontend": {
                    "gallery": {
                        "blur_sensitive_images": true
                    }
                }
            }
        })
    );
}
