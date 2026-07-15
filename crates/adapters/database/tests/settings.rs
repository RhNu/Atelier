use atelier_adapter_database::{DatabaseConnection, DatabaseSettingsRepository};
use atelier_generation::{ImageFormat, ImageModel, ImageSize, NoiseSchedule, Sampler, UcPreset};
use atelier_settings::{
    GenerationDefaults, ImageVariantSettings, WorkspaceSettings, WorkspaceSettingsRepository,
};
use futures_executor::block_on;
use rusqlite::Connection;

#[test]
fn settings_repository_defaults_saves_reopens_and_resets() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        let connection = DatabaseConnection::open(&path).unwrap();
        let repository = DatabaseSettingsRepository::new(connection);

        assert_eq!(
            repository.get_workspace_settings().await.unwrap(),
            WorkspaceSettings::default()
        );

        let settings = WorkspaceSettings {
            generation: GenerationDefaults {
                model: ImageModel::NaiDiffusion4Curated,
                size: ImageSize::square(),
                quality: false,
                uc_preset: UcPreset::None,
                steps: 31,
                scale: 7.5,
                sampler: Sampler::KDpmpp2m,
                noise_schedule: NoiseSchedule::Exponential,
                seed: 42,
                n_samples: 2,
                cfg_rescale: 0.35,
                variety_boost: true,
                image_format: Some(ImageFormat::Webp),
                strict_mode: true,
            },
            image_variants: ImageVariantSettings {
                thumbnail_long_edge: 240,
                preview_long_edge: 900,
            },
        };

        repository
            .save_workspace_settings(settings.clone())
            .await
            .unwrap();
        drop(repository);

        let reopened = DatabaseSettingsRepository::new(DatabaseConnection::open(&path).unwrap());
        assert_eq!(reopened.get_workspace_settings().await.unwrap(), settings);

        reopened.reset_workspace_settings().await.unwrap();
        assert_eq!(
            reopened.get_workspace_settings().await.unwrap(),
            WorkspaceSettings::default()
        );
    });
}

#[test]
fn settings_repository_reads_workspace_payload_without_global_fields() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atelier.sqlite3");
        {
            drop(DatabaseConnection::open(&path).unwrap());
            let connection = Connection::open(&path).unwrap();
            connection
                .execute(
                    "INSERT INTO workspace_settings(setting_key, value_json) VALUES ('workspace', ?1)",
                    [r#"{
                        "schema_version": 1,
                        "generation": {
                            "model": "nai-diffusion-4-5-full",
                            "width": 832,
                            "height": 1216,
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
                            "image_format": null,
                            "strict_mode": false
                        },
                        "image_variants": {
                            "thumbnail_long_edge": 320,
                            "preview_long_edge": 1024
                        }
                    }"#],
                )
                .unwrap();
        }

        let repository = DatabaseSettingsRepository::new(DatabaseConnection::open(&path).unwrap());
        let settings = repository.get_workspace_settings().await.unwrap();
        assert_eq!(settings, WorkspaceSettings::default());
    });
}

#[test]
fn migrations_add_settings_table_to_existing_v2_database() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("atelier.sqlite3");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                r"
                CREATE TABLE schema_migrations (
                    version INTEGER PRIMARY KEY,
                    applied_at_ms INTEGER NOT NULL DEFAULT (
                        CAST(strftime('%s', 'now') AS INTEGER) * 1000
                    )
                );
                INSERT INTO schema_migrations(version) VALUES (1);
                INSERT INTO schema_migrations(version) VALUES (2);
                ",
            )
            .unwrap();
    }

    let connection = DatabaseConnection::open(&path).unwrap();
    let repository = DatabaseSettingsRepository::new(connection.clone());
    assert_eq!(
        block_on(repository.get_workspace_settings()).unwrap(),
        WorkspaceSettings::default()
    );

    drop(connection);
    let raw = Connection::open(&path).unwrap();
    let has_v3: bool = raw
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 3)",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(has_v3);
}
