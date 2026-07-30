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
    GallerySafetyModelEvidenceDto, GallerySafetyRatingScoresDto, GallerySafetyReviewDto,
    GallerySafetyReviewStateDto, GallerySafetyRiskBandDto, GallerySafetyScanStateDto,
    GallerySourceKindDto,
};
use atelier_app_api::generation::{
    CharacterDto, CharacterPositionDto, CharacterReferenceDto, CharacterReferenceTypeDto,
    ControlNetConfigDto, ControlNetInputDto, GenerateImageRequestDto, GenerationAnlasEstimateDto,
    GenerationEstimateRequestDto, GenerationRequestStatusDto, GenerationStatusDto,
    GenerationStatusQueryDto, ImageModelDto, Img2ImgRequestDto, QueueDirectiveDto,
    RunGenerationJobRequestDto, StreamModeDto, SubmitGenerationBatchJobDto,
    SubmitGenerationBatchRequestDto,
};
use atelier_app_api::history::{
    DeleteGenerationHistoryBatchesRequestDto, DeleteGenerationHistoryBatchesResponseDto,
    DeleteRunHistoryItemsRequestDto, DeleteRunHistoryItemsResponseDto,
    GenerationBatchHistoryStatusDto, GenerationHistoryBatchDto, GenerationHistoryPageDto,
    GenerationHistoryQueryDto, RerunGenerationHistoryBatchRequestDto,
    RerunGenerationHistoryItemRequestDto, RerunGenerationHistoryItemResponseDto, RunHistoryItemDto,
    RunHistoryKindDto, RunHistoryOutputDto, RunHistoryOutputStateDto, RunHistoryPageDto,
    RunHistoryQueryDto, RunHistoryStatusDto,
};
use atelier_app_api::prompt::{
    CompileGenerationCharacterPromptDto, CompileGenerationPromptRequestDto,
    DeletePromptChunkRequestDto, DeletePromptChunkResponseDto, GetPromptChunkRequestDto,
    LexiconCompleteRequestDto, ListPromptChunksRequestDto, ListPromptPresetsRequestDto,
    PromptChunkPageDto, PromptPresetBehaviorDto, PromptPresetKindDto, UpsertPromptPresetRequestDto,
};
use atelier_app_api::resource::{
    CopyResourceImageRequestDto, GetResourceImageRequestDto, ImageExportFormatDto, ImageInputDto,
    ImageResourceKindDto, ImportImageResourceRequestDto, ImportImageResourceResponseDto,
    ReleaseImportedImageResourcesRequestDto, ReleaseImportedImageResourcesResponseDto,
    ResourceImageDto, ResourceRefDto, SaveResourceImageRequestDto, SaveResourceImagesZipEntryDto,
    SaveResourceImagesZipRequestDto,
};
use atelier_app_api::settings::{
    FrontendLanguageDto, GenerationDefaultsDto, GlobalFrontendSettingsDto,
    GlobalGallerySettingsDto, GlobalSafetySettingsDto, GlobalSettingsDto, ImageVariantSettingsDto,
    ResetWorkspaceSettingsResponseDto, UpdateGlobalSettingsRequestDto,
    UpdateWorkspaceSettingsRequestDto, WorkspaceSettingsDto,
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
            sample_index: Some(0),
            artifact_id: "artifact:job-1:sample:0".to_owned(),
            item_id: Some("gallery:job-1:sample:0".to_owned()),
            resource: Some(ResourceRefDto {
                id: "resource:job-1:sample:0".to_owned(),
                variant_id: Some("preview".to_owned()),
            }),
            asset_role: "preview".to_owned(),
            variant_kind: Some("preview".to_owned()),
            state: RunHistoryOutputStateDto::Available,
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
            request_seed: None,
            prompt: None,
            negative_prompt: None,
            embedded_metadata_status: None,
            embedded_metadata_error: None,
            embedded_metadata_warnings: Vec::new(),
            sample_index: None,
            model_name: None,
            safety: GallerySafetyDto {
                scan_state: GallerySafetyScanStateDto::Unscanned,
                risk_band: None,
                auto_label: None,
                effective_label: None,
                policy_id: None,
                policy_version: None,
                primary: None,
                review: None,
                assessed_at_ms: None,
                message: None,
            },
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
                "indexed_at_ms": 123,
                "safety": { "scan_state": "unscanned" }
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
        effective_label: Some(GallerySafetyLabelDto::Sensitive),
        policy_id: Some("anime-rating-cascade".to_owned()),
        policy_version: Some("1".to_owned()),
        primary: Some(GallerySafetyModelEvidenceDto {
            model_id: "anime_dbrating".to_owned(),
            model_revision: "revision-1".to_owned(),
            ratings: GallerySafetyRatingScoresDto {
                general: 0.09,
                sensitive: 0.0,
                questionable: 0.2,
                explicit: 0.71,
            },
            fused_score: 0.91,
        }),
        review: Some(GallerySafetyReviewDto {
            state: GallerySafetyReviewStateDto::NotNeeded,
            model_id: None,
            model_revision: None,
            evidence: None,
            message: None,
        }),
        assessed_at_ms: Some(123),
        message: None,
    };

    assert_eq!(
        serde_json::to_value(safety).unwrap(),
        json!({
            "scan_state": "scanned",
            "risk_band": "high",
            "auto_label": "sensitive",
            "effective_label": "sensitive",
            "policy_id": "anime-rating-cascade",
            "policy_version": "1",
            "primary": {
                "model_id": "anime_dbrating",
                "model_revision": "revision-1",
                "ratings": {
                    "general": 0.09_f32,
                    "sensitive": 0.0_f32,
                    "questionable": 0.2_f32,
                    "explicit": 0.71_f32
                },
                "fused_score": 0.91_f32
            },
            "review": { "state": "not_needed" },
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

#[test]
fn global_settings_dtos_separate_lifecycle_state_from_editable_preferences() {
    let settings = GlobalSettingsDto {
        last_workspace: Some("D:/atelier".into()),
        frontend: GlobalFrontendSettingsDto {
            language: FrontendLanguageDto::SimplifiedChinese,
            developer_mode: true,
            gallery: GlobalGallerySettingsDto {
                blur_sensitive_images: true,
            },
        },
        safety: GlobalSafetySettingsDto {
            wd_auto_review_enabled: true,
        },
    };

    assert_eq!(
        serde_json::to_value(&settings).unwrap(),
        json!({
            "last_workspace": "D:/atelier",
            "frontend": {
                "language": "zh-CN",
                "developer_mode": true,
                "gallery": { "blur_sensitive_images": true }
            },
            "safety": { "wd_auto_review_enabled": true }
        })
    );
    assert_eq!(
        serde_json::to_value(UpdateGlobalSettingsRequestDto {
            frontend: settings.frontend,
            safety: settings.safety,
        })
        .unwrap(),
        json!({
            "frontend": {
                "language": "zh-CN",
                "developer_mode": true,
                "gallery": { "blur_sensitive_images": true }
            },
            "safety": { "wd_auto_review_enabled": true }
        })
    );
}
