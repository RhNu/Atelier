//! Desktop system integration boundary for NAI Atelier.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesktopPaths {
    pub app_data_dir: PathBuf,
    pub app_config_dir: PathBuf,
    pub app_cache_dir: PathBuf,
    pub suggested_workspace_dir: PathBuf,
    pub resource_dir: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct DesktopSystem {
    paths: DesktopPaths,
    user_allowed_paths: Arc<Mutex<BTreeSet<PathBuf>>>,
}

impl DesktopSystem {
    #[must_use]
    pub fn new(paths: DesktopPaths) -> Self {
        Self {
            paths,
            user_allowed_paths: Arc::default(),
        }
    }

    #[must_use]
    pub const fn paths(&self) -> &DesktopPaths {
        &self.paths
    }

    /// Resolves optional NSFW scanner model and runtime assets.
    ///
    /// # Errors
    /// Returns an error when explicit environment paths are incomplete or point
    /// to missing files.
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

    /// Records a user-selected path as safe for later open or reveal actions.
    ///
    /// # Errors
    /// Returns an error when the path cannot be canonicalized or the allowlist
    /// lock is unavailable.
    pub fn allow_user_path(&self, path: impl AsRef<Path>) -> DesktopSystemResult<()> {
        let canonical = canonicalize_existing(path.as_ref())?;
        self.user_allowed_paths
            .lock()
            .map_err(|_| DesktopSystemError::new("desktop system path allowlist is unavailable"))?
            .insert(canonical);
        Ok(())
    }

    /// Opens a native directory picker for workspace roots.
    ///
    /// # Errors
    /// Returns an error when the dialog backend fails or the selected path
    /// cannot be recorded.
    pub fn pick_workspace_directory(
        &self,
        dialog: &impl DesktopFileDialog,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        let picked = dialog.pick_directory()?;
        if let Some(path) = &picked {
            self.allow_user_path(path)?;
        }
        Ok(picked)
    }

    /// Opens a native directory picker for export targets.
    ///
    /// # Errors
    /// Returns an error when the dialog backend fails or the selected path
    /// cannot be recorded.
    pub fn pick_export_directory(
        &self,
        dialog: &impl DesktopFileDialog,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        self.pick_workspace_directory(dialog)
    }

    /// Opens a native file picker for image resources.
    ///
    /// # Errors
    /// Returns an error when the dialog backend fails or a selected path cannot
    /// be recorded.
    pub fn pick_image_files(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        self.pick_files(dialog, options)
    }

    /// Opens a native file picker for Vibe documents.
    ///
    /// # Errors
    /// Returns an error when the dialog backend fails or a selected path cannot
    /// be recorded.
    pub fn pick_vibe_documents(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        self.pick_files(dialog, options)
    }

    /// Opens a path after validating it is app-owned or user-selected.
    ///
    /// # Errors
    /// Returns an error when the path is outside the allowed scope or the opener
    /// backend fails.
    pub fn open_path(
        &self,
        path: impl AsRef<Path>,
        opener: &impl DesktopPathOpener,
    ) -> DesktopSystemResult<()> {
        let path = path.as_ref();
        self.ensure_open_allowed(path)?;
        opener.open_path(path)
    }

    /// Reveals a path after validating it is app-owned or user-selected.
    ///
    /// # Errors
    /// Returns an error when the path is outside the allowed scope or the opener
    /// backend fails.
    pub fn reveal_path(
        &self,
        path: impl AsRef<Path>,
        opener: &impl DesktopPathOpener,
    ) -> DesktopSystemResult<()> {
        let path = path.as_ref();
        self.ensure_open_allowed(path)?;
        opener.reveal_path(path)
    }

    /// Sends a desktop notification through the host backend.
    ///
    /// # Errors
    /// Returns an error when the notification backend rejects the request.
    pub fn notify(
        &self,
        title: &str,
        body: &str,
        notifier: &impl DesktopNotifier,
    ) -> DesktopSystemResult<()> {
        notifier.notify(title, body)
    }

    fn pick_files(
        &self,
        dialog: &impl DesktopFileDialog,
        options: PickFilesOptions,
    ) -> DesktopSystemResult<Vec<PathBuf>> {
        let files = dialog.pick_files(options)?;
        for path in &files {
            self.allow_user_path(path)?;
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

    fn is_user_allowed_path(&self, canonical: &Path) -> DesktopSystemResult<bool> {
        Ok(self
            .user_allowed_paths
            .lock()
            .map_err(|_| DesktopSystemError::new("desktop system path allowlist is unavailable"))?
            .iter()
            .any(|allowed| canonical == allowed || canonical.starts_with(allowed)))
    }

    fn is_app_owned_path(&self, canonical: &Path) -> bool {
        for root in self.app_owned_roots() {
            let Ok(root) = root.canonicalize() else {
                continue;
            };
            if canonical == root || canonical.starts_with(root) {
                return true;
            }
        }
        false
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
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickFilesOptions {
    pub extensions: Vec<String>,
}

pub trait DesktopPathOpener {
    /// Opens a path through the host OS.
    ///
    /// # Errors
    /// Returns an error when the host cannot open the path.
    fn open_path(&self, path: &Path) -> DesktopSystemResult<()>;

    /// Reveals a path in the host file manager.
    ///
    /// # Errors
    /// Returns an error when the host cannot reveal the path.
    fn reveal_path(&self, path: &Path) -> DesktopSystemResult<()>;
}

pub trait DesktopFileDialog {
    /// Opens a directory picker.
    ///
    /// # Errors
    /// Returns an error when the host dialog fails.
    fn pick_directory(&self) -> DesktopSystemResult<Option<PathBuf>>;

    /// Opens a multi-file picker.
    ///
    /// # Errors
    /// Returns an error when the host dialog fails.
    fn pick_files(&self, options: PickFilesOptions) -> DesktopSystemResult<Vec<PathBuf>>;
}

pub trait DesktopNotifier {
    /// Sends a desktop notification.
    ///
    /// # Errors
    /// Returns an error when the host notification backend fails.
    fn notify(&self, title: &str, body: &str) -> DesktopSystemResult<()>;
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
