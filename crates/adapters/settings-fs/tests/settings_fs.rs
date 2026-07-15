use atelier_adapter_settings_fs::FileSystemGlobalSettingsRepository;
use atelier_settings::{
    GlobalFrontendSettings, GlobalGallerySettings, GlobalSettings, GlobalSettingsRepository,
};
use futures_executor::block_on;

#[test]
fn missing_file_returns_defaults_and_round_trips_settings() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config").join("global-settings.json");
        let repository = FileSystemGlobalSettingsRepository::new(&path);

        assert_eq!(
            repository.get_global_settings().await.unwrap(),
            GlobalSettings::default()
        );

        let settings = GlobalSettings {
            last_workspace: Some(temp.path().join("workspace")),
            frontend: GlobalFrontendSettings {
                developer_mode: true,
                gallery: GlobalGallerySettings {
                    blur_sensitive_images: true,
                },
            },
        };
        repository
            .save_global_settings(settings.clone())
            .await
            .unwrap();

        assert_eq!(repository.get_global_settings().await.unwrap(), settings);

        let mut updated = settings;
        updated.frontend.gallery.blur_sensitive_images = false;
        repository
            .save_global_settings(updated.clone())
            .await
            .unwrap();
        assert_eq!(repository.get_global_settings().await.unwrap(), updated);
    });
}

#[test]
fn missing_developer_mode_defaults_to_false_without_quarantining_settings() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("global-settings.json");
        std::fs::write(
            &path,
            r#"{
  "schema_version": 1,
  "last_workspace": null,
  "frontend": {
    "gallery": { "blur_sensitive_images": true }
  }
}"#,
        )
        .unwrap();
        let repository = FileSystemGlobalSettingsRepository::new(&path);

        let settings = repository.get_global_settings().await.unwrap();
        assert!(!settings.frontend.developer_mode);
        assert!(settings.frontend.gallery.blur_sensitive_images);
        assert!(path.exists());
    });
}

#[test]
fn invalid_file_is_quarantined_and_defaults_are_returned() {
    block_on(async {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("global-settings.json");
        std::fs::write(&path, "not json").unwrap();
        let repository = FileSystemGlobalSettingsRepository::new(&path);

        assert_eq!(
            repository.get_global_settings().await.unwrap(),
            GlobalSettings::default()
        );
        assert!(!path.exists());
        assert!(std::fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("global-settings.json.invalid-")
        }));
    });
}
