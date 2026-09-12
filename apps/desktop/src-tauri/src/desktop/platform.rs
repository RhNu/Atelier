use crate::desktop_system::{
    DesktopFileDialog, DesktopNotifier, DesktopPathOpener, DesktopSystemError, DesktopSystemResult,
    PickFilesOptions,
};
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
pub struct TauriDialog {
    app_handle: AppHandle,
}

impl TauriDialog {
    #[must_use]
    pub const fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }

    pub fn save_file(
        &self,
        default_file_name: Option<&str>,
        extension: Option<&str>,
    ) -> DesktopSystemResult<Option<PathBuf>> {
        let mut builder = self.app_handle.dialog().file();
        if let Some(default_file_name) = default_file_name {
            builder = builder.set_file_name(default_file_name);
        }
        if let Some(extension) = extension {
            builder = builder.add_filter("Files", &[extension]);
        }

        builder
            .blocking_save_file()
            .map(file_path_to_path)
            .transpose()
    }
}

impl DesktopFileDialog for TauriDialog {
    fn pick_directory(&self) -> DesktopSystemResult<Option<PathBuf>> {
        self.app_handle
            .dialog()
            .file()
            .blocking_pick_folder()
            .map(file_path_to_path)
            .transpose()
    }

    fn pick_files(&self, options: PickFilesOptions) -> DesktopSystemResult<Vec<PathBuf>> {
        let mut builder = self.app_handle.dialog().file();
        if !options.extensions.is_empty() {
            let extension_refs: Vec<&str> = options.extensions.iter().map(String::as_str).collect();
            builder = builder.add_filter("Files", &extension_refs);
        }
        let Some(files) = builder.blocking_pick_files() else {
            return Ok(Vec::new());
        };
        files.into_iter().map(file_path_to_path).collect()
    }
}

fn file_path_to_path(path: FilePath) -> DesktopSystemResult<PathBuf> {
    path.into_path().map_err(|error| {
        DesktopSystemError::new(format!(
            "selected path is not a local filesystem path: {error}"
        ))
    })
}

pub struct TauriPathOpener {
    app_handle: AppHandle,
}

impl TauriPathOpener {
    #[must_use]
    pub const fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl DesktopPathOpener for TauriPathOpener {
    fn open_path(&self, path: &Path) -> DesktopSystemResult<()> {
        self.app_handle
            .opener()
            .open_path(path.to_string_lossy().into_owned(), None::<String>)
            .map_err(|error| DesktopSystemError::new(format!("failed to open path: {error}")))
    }

    fn reveal_path(&self, path: &Path) -> DesktopSystemResult<()> {
        self.app_handle
            .opener()
            .reveal_item_in_dir(path)
            .map_err(|error| DesktopSystemError::new(format!("failed to reveal path: {error}")))
    }
}

pub struct TauriNotifier {
    app_handle: AppHandle,
}

impl TauriNotifier {
    #[must_use]
    pub const fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
}

impl DesktopNotifier for TauriNotifier {
    fn notify(&self, title: &str, body: &str) -> DesktopSystemResult<()> {
        self.app_handle
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|error| {
                DesktopSystemError::new(format!("failed to send notification: {error}"))
            })
    }
}
