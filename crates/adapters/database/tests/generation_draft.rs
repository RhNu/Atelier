use atelier_adapter_database::{
    DatabaseConnection, DatabaseGenerationDraftRepository, DatabaseSettingsRepository,
};
use atelier_generation::{
    CharacterPosition, CharacterReferenceType, GenerationDraftCharacter,
    GenerationDraftCharacterPositionMode, GenerationDraftI2i, GenerationDraftPreciseReference,
    GenerationDraftRepository, GenerationDraftSeedMode, GenerationDraftSnapshot,
    GenerationDraftVibe, GenerationDraftVibeSlot, ImageFormat, ImageModel, ImageSize,
    NoiseSchedule, Sampler, UcPreset,
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
        let draft = sample_draft();
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

fn sample_draft() -> GenerationDraftSnapshot {
    GenerationDraftSnapshot {
        main_preset_id: Some("main-preset".to_owned()),
        prompt: "1girl, cinematic lighting".to_owned(),
        negative_prompt: "low quality".to_owned(),
        model: ImageModel::NaiDiffusion4Curated,
        size: ImageSize {
            width: 1024,
            height: 1536,
        },
        quality: false,
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
            mask: Some(resource("resource:mask")),
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
        characters: vec![GenerationDraftCharacter {
            id: "character-slot".to_owned(),
            preset_id: Some("character-preset".to_owned()),
            prompt: "red hair".to_owned(),
            negative_prompt: "hat".to_owned(),
            enabled: true,
            position: CharacterPosition { x: 0.2, y: 0.8 },
        }],
        character_position_mode: GenerationDraftCharacterPositionMode::Manual,
    }
}

fn resource(id: &str) -> ResourceRef {
    ResourceRef::base(ResourceId::new(id))
}
