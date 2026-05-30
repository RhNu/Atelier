//! Workspace-local settings domain.

mod error;
mod model;
mod ports;
mod service;

pub use error::{SettingsError, SettingsErrorKind, SettingsResult};
pub use model::{
    FrontendGallerySettings, FrontendSettings, GenerationDefaults, ImageVariantSettings,
    WorkspaceSettings,
};
pub use ports::SettingsRepository;
pub use service::SettingsService;
