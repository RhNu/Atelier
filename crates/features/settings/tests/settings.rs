use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use atelier_generation::{ImageFormat, ImageModel, ImageSize, NoiseSchedule, Sampler, UcPreset};
use atelier_settings::{
    FrontendLanguage, GenerationDefaults, GlobalFrontendSettings, GlobalGallerySettings,
    GlobalSettings, GlobalSettingsRepository, GlobalSettingsService, ImageVariantSettings,
    SettingsResult, WorkspaceSettings, WorkspaceSettingsRepository, WorkspaceSettingsService,
};
use futures_executor::block_on;

#[test]
fn workspace_settings_defaults_are_novelai_oriented() {
    let settings = WorkspaceSettings::default();

    assert_eq!(settings.generation.model, ImageModel::NaiDiffusion45Full);
    assert_eq!(settings.generation.size, ImageSize::portrait());
    assert_eq!(settings.generation.steps, 23);
    assert!((settings.generation.scale - 5.0).abs() < f32::EPSILON);
    assert_eq!(settings.generation.n_samples, 1);
    assert_eq!(settings.generation.sampler, Sampler::KEulerAncestral);
    assert_eq!(settings.generation.noise_schedule, NoiseSchedule::Karras);
    assert_eq!(settings.generation.uc_preset, UcPreset::Light);
    assert_eq!(settings.generation.image_format, None);
    assert_eq!(settings.image_variants.thumbnail_long_edge, 320);
    assert_eq!(settings.image_variants.preview_long_edge, 1024);
}

#[test]
fn settings_validation_rejects_invalid_variant_and_generation_scalars() {
    let mut settings = WorkspaceSettings::default();
    settings.image_variants.thumbnail_long_edge = 0;
    assert!(settings.validate().is_err());

    let mut settings = WorkspaceSettings::default();
    settings.image_variants.preview_long_edge = 4097;
    assert!(settings.validate().is_err());

    let mut settings = WorkspaceSettings::default();
    settings.generation.scale = f32::NAN;
    assert!(settings.validate().is_err());

    let mut settings = WorkspaceSettings::default();
    settings.generation.steps = 51;
    assert!(settings.validate().is_err());
}

#[test]
fn settings_service_defaults_saves_and_resets_workspace_settings() {
    block_on(async {
        let repository = MemoryWorkspaceSettingsRepository::default();
        let service = WorkspaceSettingsService::new(repository.clone());

        assert_eq!(
            service.get_workspace_settings().await.unwrap(),
            WorkspaceSettings::default()
        );

        let custom = WorkspaceSettings {
            generation: GenerationDefaults {
                model: ImageModel::NaiDiffusion4Curated,
                size: ImageSize::square(),
                quality: false,
                uc_preset: UcPreset::None,
                steps: 28,
                scale: 6.5,
                sampler: Sampler::KDpmpp2m,
                noise_schedule: NoiseSchedule::Exponential,
                seed: 123,
                n_samples: 2,
                cfg_rescale: 0.4,
                variety_boost: true,
                image_format: Some(ImageFormat::Webp),
                strict_mode: true,
            },
            image_variants: ImageVariantSettings {
                thumbnail_long_edge: 256,
                preview_long_edge: 768,
            },
        };

        assert_eq!(
            service
                .update_workspace_settings(custom.clone())
                .await
                .unwrap(),
            custom
        );
        assert_eq!(service.get_workspace_settings().await.unwrap(), custom);

        assert_eq!(
            service.reset_workspace_settings().await.unwrap(),
            WorkspaceSettings::default()
        );
        assert_eq!(
            service.get_workspace_settings().await.unwrap(),
            WorkspaceSettings::default()
        );
    });
}

#[test]
fn global_settings_service_preserves_lifecycle_state_when_updating_frontend() {
    block_on(async {
        let repository = MemoryGlobalSettingsRepository::default();
        let service = GlobalSettingsService::new(Arc::new(repository));

        let workspace = std::path::PathBuf::from("D:/atelier");
        service
            .record_last_workspace(workspace.clone())
            .await
            .unwrap();
        let settings = service
            .update_frontend_settings(GlobalFrontendSettings {
                language: FrontendLanguage::SimplifiedChinese,
                developer_mode: true,
                gallery: GlobalGallerySettings {
                    blur_sensitive_images: true,
                },
            })
            .await
            .unwrap();

        assert_eq!(settings.last_workspace, Some(workspace));
        assert_eq!(
            settings.frontend.language,
            FrontendLanguage::SimplifiedChinese
        );
        assert!(settings.frontend.developer_mode);
        assert!(settings.frontend.gallery.blur_sensitive_images);
    });
}

#[derive(Clone, Default)]
struct MemoryWorkspaceSettingsRepository {
    state: Arc<Mutex<Option<WorkspaceSettings>>>,
}

#[async_trait]
impl WorkspaceSettingsRepository for MemoryWorkspaceSettingsRepository {
    async fn get_workspace_settings(&self) -> SettingsResult<WorkspaceSettings> {
        Ok(self.state.lock().unwrap().clone().unwrap_or_default())
    }

    async fn save_workspace_settings(&self, settings: WorkspaceSettings) -> SettingsResult<()> {
        *self.state.lock().unwrap() = Some(settings);
        Ok(())
    }

    async fn reset_workspace_settings(&self) -> SettingsResult<()> {
        *self.state.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MemoryGlobalSettingsRepository {
    state: Arc<Mutex<GlobalSettings>>,
}

#[async_trait]
impl GlobalSettingsRepository for MemoryGlobalSettingsRepository {
    async fn get_global_settings(&self) -> SettingsResult<GlobalSettings> {
        Ok(self.state.lock().unwrap().clone())
    }

    async fn save_global_settings(&self, settings: GlobalSettings) -> SettingsResult<()> {
        *self.state.lock().unwrap() = settings;
        Ok(())
    }
}
