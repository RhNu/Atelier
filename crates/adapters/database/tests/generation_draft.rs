use atelier_adapter_database::{
    DatabaseConnection, DatabaseGenerationDraftRepository, DatabaseSettingsRepository,
};
use atelier_generation::{
    CharacterPosition, CharacterReferenceType, GenerationDraftCharacter,
    GenerationDraftCharacterPositionMode, GenerationDraftFocusRegion, GenerationDraftI2i,
    GenerationDraftInpaintSession, GenerationDraftMaskDisplay, GenerationDraftPreciseReference,
    GenerationDraftPromptState, GenerationDraftReferenceInset, GenerationDraftRepository,
    GenerationDraftSeedMode, GenerationDraftSnapshot, GenerationDraftVibe, GenerationDraftVibeSlot,
    ImageFormat, ImageModel, ImageSize, NoiseSchedule, QualityPreset, Sampler, UcPreset,
};
use atelier_resource_catalog::{ResourceId, ResourceRef};
use atelier_settings::{WorkspaceSettings, WorkspaceSettingsRepository};
use futures_executor::block_on;
use rusqlite::Connection;

#[test]
fn draft_round_trips_all_fields_without_overwriting_workspace_settings() {
    block_on(async {
        let connection = DatabaseConnection::open_memory().unwrap();
        let draft_repository = DatabaseGenerationDraftRepository::new(connection.clone());
        let settings_repository = DatabaseSettingsRepository::new(connection);
        let settings = WorkspaceSettings::default();
        settings_repository
            .save_workspace_settings(settings.clone())
            .await
            .unwrap();

        assert_eq!(
            draft_repository.load_generation_draft().await.unwrap(),
            None
        );
        let mut draft = sample_draft();
        draft.i2i.as_mut().unwrap().inpaint.as_mut().unwrap().focus =
            Some(GenerationDraftFocusRegion {
                x: 0.25,
                y: 0.25,
                width: 0.5,
                height: 0.5,
                minimum_context_area: 0.25,
            });
        draft
            .i2i
            .as_mut()
            .unwrap()
            .inpaint
            .as_mut()
            .unwrap()
            .reference_insets
            .push(GenerationDraftReferenceInset {
                id: "reference-inset".to_owned(),
                image: resource("resource:inset"),
                x: 0.1,
                y: 0.2,
                width: 0.3,
                height: 0.3,
                border_enabled: true,
                border_width: 4,
            });
        draft_repository
            .save_generation_draft(&draft)
            .await
            .unwrap();
        assert_eq!(
            draft_repository.load_generation_draft().await.unwrap(),
            Some(draft)
        );
        assert_eq!(
            settings_repository.get_workspace_settings().await.unwrap(),
            settings
        );

        draft_repository.clear_generation_draft().await.unwrap();
        assert_eq!(
            draft_repository.load_generation_draft().await.unwrap(),
            None
        );
        assert_eq!(
            settings_repository.get_workspace_settings().await.unwrap(),
            settings
        );
    });
}

#[test]
fn draft_reports_corrupt_and_unknown_schema_payloads() {
    for payload in ["not-json", r#"{"schema_version":99}"#] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        drop(DatabaseConnection::open(&path).unwrap());
        Connection::open(&path)
            .unwrap()
            .execute(
                "INSERT INTO workspace_settings(setting_key, value_json) VALUES ('generation.draft', ?1)",
                [payload],
            )
            .unwrap();

        let repository =
            DatabaseGenerationDraftRepository::new(DatabaseConnection::open(&path).unwrap());
        let error = block_on(repository.load_generation_draft()).unwrap_err();
        assert!(error.to_string().contains("generation_draft_repository"));
        block_on(repository.clear_generation_draft()).unwrap();
        assert_eq!(block_on(repository.load_generation_draft()).unwrap(), None);
    }
}

#[test]
fn draft_migrates_schema_v2_mask_to_semantic_inpaint_session() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let repository =
            DatabaseGenerationDraftRepository::new(DatabaseConnection::open(&path).unwrap());
        let expected = sample_draft();
        repository.save_generation_draft(&expected).await.unwrap();

        let raw_connection = Connection::open(&path).unwrap();
        let raw: String = raw_connection
            .query_row(
                "SELECT value_json FROM workspace_settings WHERE setting_key = 'generation.draft'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
        value["schema_version"] = serde_json::json!(2);
        let i2i = value["i2i"].as_object_mut().unwrap();
        let inpaint = i2i.remove("inpaint").unwrap();
        i2i.insert("mask".to_owned(), inpaint["region_to_replace"].clone());
        raw_connection
            .execute(
                "UPDATE workspace_settings SET value_json = ?1 WHERE setting_key = 'generation.draft'",
                [serde_json::to_string(&value).unwrap()],
            )
            .unwrap();

        assert_eq!(
            repository.load_generation_draft().await.unwrap(),
            Some(expected)
        );
    });
}

fn sample_draft() -> GenerationDraftSnapshot {
    GenerationDraftSnapshot {
        model: ImageModel::NaiDiffusion4Curated,
        prompt_states: vec![GenerationDraftPromptState {
            model: ImageModel::NaiDiffusion4Curated,
            main_preset_id: Some("main-preset".to_owned()),
            prompt: "1girl, cinematic lighting".to_owned(),
            negative_prompt: "low quality".to_owned(),
            furry_mode: true,
            characters: vec![GenerationDraftCharacter {
                id: "character-slot".to_owned(),
                preset_id: Some("character-preset".to_owned()),
                prompt: "red hair".to_owned(),
                negative_prompt: "hat".to_owned(),
                enabled: true,
                position: CharacterPosition { x: 0.2, y: 0.8 },
            }],
            character_position_mode: GenerationDraftCharacterPositionMode::Manual,
        }],
        size: ImageSize {
            width: 1024,
            height: 1536,
        },
        quality: QualityPreset::None,
        transparent_background: false,
        uc_preset: UcPreset::None,
        steps: 31,
        scale: 7.5,
        sampler: Sampler::KDpmppSde,
        noise_schedule: NoiseSchedule::Polyexponential,
        seed_mode: GenerationDraftSeedMode::Fixed,
        seed: 9_223_372,
        n_samples: 4,
        request_count: 8,
        cfg_rescale: 0.35,
        variety_boost: true,
        image_format: Some(ImageFormat::Webp),
        strict_mode: true,
        stream_enabled: false,
        i2i: Some(GenerationDraftI2i {
            image: resource("resource:i2i"),
            inpaint: Some(GenerationDraftInpaintSession {
                region_to_replace: resource("resource:mask"),
                display: GenerationDraftMaskDisplay::default(),
                focus: None,
                reference_insets: Vec::new(),
            }),
            strength: 0.55,
            noise: 0.15,
        }),
        vibe: GenerationDraftVibe {
            enabled: false,
            strength: 0.75,
            slots: vec![GenerationDraftVibeSlot {
                id: "vibe-slot".to_owned(),
                encoding: resource("resource:vibe-encoding"),
                vibe_id: Some("vibe-document".to_owned()),
                information_extracted: 0.65,
                strength: 0.85,
                display_name: "Warm film".to_owned(),
                source_image: Some(resource("resource:vibe-source")),
                source_sha256: Some("abc123".to_owned()),
                model: ImageModel::NaiDiffusion4Curated,
            }],
        },
        precise_references: vec![GenerationDraftPreciseReference {
            id: "reference-slot".to_owned(),
            image: resource("resource:reference"),
            reference_type: CharacterReferenceType::Style,
            fidelity: 0.45,
            strength: 0.7,
            display_name: "Painterly".to_owned(),
        }],
    }
}

fn resource(id: &str) -> ResourceRef {
    ResourceRef::base(ResourceId::new(id))
}
