//! User-level and workspace-local settings domains.

mod error;
mod model;
mod ports;
mod service;

pub use error::{SettingsError, SettingsErrorKind, SettingsResult};
pub use model::{
    FrontendLanguage, GenerationDefaults, GlobalFrontendSettings, GlobalGallerySettings,
    GlobalSettings, ImageVariantSettings, WorkspaceSettings,
};
pub use ports::{GlobalSettingsRepository, WorkspaceSettingsRepository};
pub use service::{GlobalSettingsService, WorkspaceSettingsService};
