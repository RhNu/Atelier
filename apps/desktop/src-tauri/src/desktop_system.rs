use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use nai_atelier_adapter_safety_onnx::NsfwRuntimeAssets;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type DesktopSystemResult<T> = Result<T, DesktopSystemError>;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct DesktopSystemError {
    message: String,
}

impl DesktopSystemError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct DesktopPaths {
    pub app_data_dir: PathBuf,
    pub app_config_dir: PathBuf,
    pub app_cache_dir: PathBuf,
    pub suggested_workspace_dir: PathBuf,
    pub resource_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickFilesOptions {
    pub extensions: Vec<String>,
}

pub trait DesktopPathOpener {
    fn open_path(&self, path: &Path) -> DesktopSystemResult<()>;

    fn reveal_path(&self, path: &Path) -> DesktopSystemResult<()>;
}

pub trait DesktopFileDialog {
    fn pick_directory(&self) -> DesktopSystemResult<Option<PathBuf>>;

    fn pick_files(&self, options: PickFilesOptions) -> DesktopSystemResult<Vec<PathBuf>>;
}

pub trait DesktopNotifier {
    fn notify(&self, title: &str, body: &str) -> DesktopSystemResult<()>;
}

#[derive(Clone, Debug)]
pub struct DesktopSystem {
    paths: DesktopPaths,
    user_allowed_paths: Arc<Mutex<BTreeSet<PathBuf>>>,
}

impl DesktopSystem {
    pub fn new(paths: DesktopPaths) -> Self {
        Self {
            paths,
            user_allowed_paths: Arc::default(),
        }
    }

    pub const fn paths(&self) -> &DesktopPaths {
        &self.paths
    }

    pub fn resolve_safety_assets(&self) -> DesktopSystemResult<Option<NsfwRuntimeAssets>> {
        if let Some(model_path) = env_path("NAI_ATELIER_SAFETY_ONNX_MODEL") {
            let runtime_library_path = env_path("NAI_ATELIER_ONNX_RUNTIME").ok_or_else(|| {
                DesktopSystemError::new(
                    "NAI_ATELIER_ONNX_RUNTIME must be set when safety model env is set",
                )
            })?;
            require_file(&model_path, "safety model")?;
            require_file(&runtime_library_path, "ONNX Runtime library")?;
            return Ok(Some(NsfwRuntimeAssets {
                model_path,
                runtime_library_path: Some(runtime_library_path),
            }));
        }

        let Some(resource_dir) = &self.paths.resource_dir else {
            return Ok(None);
        };
        let safety_dir = resource_dir.join("safety");
        let model_path = safety_dir.join("open_nsfw.onnx");
        let runtime_library_path = safety_dir.join(runtime_library_file_name());
        if model_path.exists() && runtime_library_path.exists() {
            return Ok(Some(NsfwRuntimeAssets {
                model_path,
                runtime_library_path: Some(runtime_library_path),
            }));
        }
        Ok(None)
    }

    pub fn allow_user_path(&self, path: impl AsRef<Path>) -> DesktopSystemResult<()> {
        let canonical = canonicalize_existing(path.as_ref())?;
        self.user_allowed_paths
            .lock()
            .map_err(|_| DesktopSystemError::new("desktop system path allowlist is unavailable"))?
            .insert(canonical);
        Ok(())
    }

    pub fn pick_workspace_directory(
        &self,
        dialog: &impl DesktopFileDialog,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        self.pick_directory(dialog)
    }

    pub fn pick_export_directory(
        &self,
        dialog: &impl DesktopFileDialog,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        self.pick_directory(dialog)
    }

    pub fn pick_image_files(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        let options = if options.extensions.is_empty() {
            PickFilesOptions {
                extensions: vec![
                    "png".to_owned(),
                    "jpg".to_owned(),
                    "jpeg".to_owned(),
                    "webp".to_owned(),
                ],
            }
        } else {
            options
        };
        self.pick_files(dialog, options)
    }

    pub fn pick_vibe_documents(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        let options = if options.extensions.is_empty() {
            PickFilesOptions {
                extensions: vec![
                    "naiv4vibe".to_owned(),
                    "naiv4vibebundle".to_owned(),
                    "json".to_owned(),
                ],
            }
        } else {
            options
        };
        self.pick_files(dialog, options)
    }

    pub fn pick_png_files(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        let options = if options.extensions.is_empty() {
            PickFilesOptions {
                extensions: vec!["png".to_owned()],
            }
        } else {
            options
        };
        self.pick_files(dialog, options)
    }

    pub fn open_path(
        &self,
        path: impl AsRef<Path>,
        opener: &impl DesktopPathOpener,
    ) -> DesktopSystemResult<()> {
        let path = path.as_ref();
        self.ensure_open_allowed(path)?;
        opener.open_path(path)
    }

    pub fn reveal_path(
        &self,
        path: impl AsRef<Path>,
        opener: &impl DesktopPathOpener,
    ) -> DesktopSystemResult<()> {
        let path = path.as_ref();
        self.ensure_open_allowed(path)?;
        opener.reveal_path(path)
    }

    #[allow(clippy::unused_self)]
    pub fn notify(
        &self,
        title: &str,
        body: &str,
        notifier: &impl DesktopNotifier,
    ) -> DesktopSystemResult<()> {
        notifier.notify(title, body)
    }

    fn pick_directory(
        &self,
        dialog: &impl DesktopFileDialog,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        let Some(path) = dialog.pick_directory()? else {
            return Ok(None);
        };
        self.allow_user_path(&path)?;
        Ok(Some(path))
    }

    fn pick_files(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        let files = dialog.pick_files(options)?;
        for file in &files {
            self.allow_user_path(file)?;
        }
        Ok(files)
    }

    fn ensure_open_allowed(&self, path: &Path) -> DesktopSystemResult<()> {
        let canonical = canonicalize_existing(path)?;
        if self.is_app_owned_path(&canonical) || self.is_user_allowed_path(&canonical)? {
            return Ok(());
        }

        Err(DesktopSystemError::new(format!(
            "desktop path is not app-owned or user-selected: {}",
            path.display()
        )))
    }

    fn is_user_allowed_path(&self, path: &Path) -> DesktopSystemResult<bool> {
        let guard = self
            .user_allowed_paths
            .lock()
            .map_err(|_| DesktopSystemError::new("desktop system path allowlist is unavailable"))?;
        Ok(guard
            .iter()
            .any(|allowed| path == allowed || path.starts_with(allowed)))
    }

    fn is_app_owned_path(&self, path: &Path) -> bool {
        self.app_owned_roots()
            .iter()
            .any(|root| path.starts_with(root))
    }

    fn app_owned_roots(&self) -> Vec<PathBuf> {
        let mut roots = vec![
            self.paths.app_data_dir.clone(),
            self.paths.app_config_dir.clone(),
            self.paths.app_cache_dir.clone(),
            self.paths.suggested_workspace_dir.clone(),
        ];
        if let Some(resource_dir) = &self.paths.resource_dir {
            roots.push(resource_dir.clone());
        }
        roots
            .into_iter()
            .filter_map(|root| canonicalize_existing(&root).ok())
            .collect()
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn require_file(path: &Path, label: &str) -> DesktopSystemResult<()> {
    if path.exists() {
        Ok(())
    } else {
        Err(DesktopSystemError::new(format!(
            "{label} file is missing: {}",
            path.display()
        )))
    }
}

fn canonicalize_existing(path: &Path) -> DesktopSystemResult<PathBuf> {
    path.canonicalize().map_err(|error| {
        DesktopSystemError::new(format!(
            "failed to resolve path {}: {error}",
            path.display()
        ))
    })
}

#[cfg(target_os = "windows")]
const fn runtime_library_file_name() -> &'static str {
    "onnxruntime.dll"
}

#[cfg(target_os = "linux")]
const fn runtime_library_file_name() -> &'static str {
    "libonnxruntime.so"
}

#[cfg(target_os = "macos")]
const fn runtime_library_file_name() -> &'static str {
    "libonnxruntime.dylib"
}

#[cfg(test)]
#[path = "desktop_system_tests.rs"]
mod tests;
