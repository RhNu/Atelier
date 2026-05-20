use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use nai_atelier_adapter_desktop_system::{
    DesktopFileDialog, DesktopNotifier, DesktopPathOpener, DesktopPaths, DesktopSystem,
    PickFilesOptions,
};

#[test]
fn safety_assets_prefer_environment_paths() {
    let _guard = env_guard();
    let temp = tempfile::tempdir().unwrap();
    let model = touch(temp.path().join("env-model.onnx"));
    let runtime = touch(temp.path().join(runtime_library_file_name()));
    set_env("NAI_ATELIER_SAFETY_ONNX_MODEL", Some(&model));
    set_env("NAI_ATELIER_ONNX_RUNTIME", Some(&runtime));

    let system = DesktopSystem::new(paths_with_resource_dir(temp.path().join("resources")));
    let assets = system.resolve_safety_assets().unwrap().unwrap();

    assert_eq!(assets.model_path, model);
    assert_eq!(
        assets.runtime_library_path.as_deref(),
        Some(runtime.as_path())
    );
}

#[test]
fn safety_assets_fall_back_to_bundled_resource_paths() {
    let _guard = env_guard();
    set_env("NAI_ATELIER_SAFETY_ONNX_MODEL", None);
    set_env("NAI_ATELIER_ONNX_RUNTIME", None);
    let temp = tempfile::tempdir().unwrap();
    let resource_dir = temp.path().join("resources");
    let safety_dir = resource_dir.join("safety");
    std::fs::create_dir_all(&safety_dir).unwrap();
    let model = touch(safety_dir.join("open_nsfw.onnx"));
    let runtime = touch(safety_dir.join(runtime_library_file_name()));

    let system = DesktopSystem::new(paths_with_resource_dir(resource_dir));
    let assets = system.resolve_safety_assets().unwrap().unwrap();

    assert_eq!(assets.model_path, model);
    assert_eq!(
        assets.runtime_library_path.as_deref(),
        Some(runtime.as_path())
    );
}

#[test]
fn safety_assets_are_optional_when_missing() {
    let _guard = env_guard();
    set_env("NAI_ATELIER_SAFETY_ONNX_MODEL", None);
    set_env("NAI_ATELIER_ONNX_RUNTIME", None);
    let temp = tempfile::tempdir().unwrap();

    let system = DesktopSystem::new(paths_with_resource_dir(temp.path().join("resources")));

    assert!(system.resolve_safety_assets().unwrap().is_none());
}

#[test]
fn open_and_reveal_reject_paths_outside_app_roots_or_user_picks() {
    let temp = tempfile::tempdir().unwrap();
    let app_data = temp.path().join("app-data");
    let outside = temp.path().join("outside.png");
    std::fs::create_dir_all(&app_data).unwrap();
    touch(&outside);
    let system = DesktopSystem::new(paths_with_app_data(app_data));
    let opener = RecordingOpener::default();

    assert!(system.open_path(&outside, &opener).is_err());
    assert!(system.reveal_path(&outside, &opener).is_err());
    assert!(opener.calls().is_empty());
}

#[test]
fn open_and_reveal_allow_app_owned_and_user_selected_paths() {
    let temp = tempfile::tempdir().unwrap();
    let app_data = temp.path().join("app-data");
    let app_owned = touch(app_data.join("gallery").join("image.png"));
    let selected = touch(temp.path().join("selected.png"));
    let system = DesktopSystem::new(paths_with_app_data(app_data));
    system.allow_user_path(selected.clone()).unwrap();
    let opener = RecordingOpener::default();

    system.open_path(&app_owned, &opener).unwrap();
    system.reveal_path(&selected, &opener).unwrap();

    assert_eq!(
        opener.calls(),
        vec![
            format!("open:{}", app_owned.display()),
            format!("reveal:{}", selected.display())
        ]
    );
}

#[test]
fn dialog_picks_are_recorded_as_user_allowed_paths() {
    let temp = tempfile::tempdir().unwrap();
    let picked_dir = touch_dir(temp.path().join("workspace"));
    let picked_file = touch(temp.path().join("source.png"));
    let dialog = RecordingDialog {
        directory: Some(picked_dir.clone()),
        files: vec![picked_file.clone()],
    };
    let system = DesktopSystem::new(paths_with_app_data(temp.path().join("app-data")));
    let opener = RecordingOpener::default();

    assert_eq!(
        system.pick_workspace_directory(&dialog).unwrap(),
        Some(picked_dir.clone())
    );
    assert_eq!(
        system
            .pick_image_files(&dialog, PickFilesOptions::default())
            .unwrap(),
        vec![picked_file.clone()]
    );
    system.open_path(&picked_dir, &opener).unwrap();
    system.reveal_path(&picked_file, &opener).unwrap();

    assert_eq!(opener.calls().len(), 2);
}

#[test]
fn opened_workspace_root_can_be_recorded_without_picker() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = touch_dir(temp.path().join("external-workspace"));
    let workspace_file = touch(workspace.join("workspace.json"));
    let system = DesktopSystem::new(paths_with_app_data(temp.path().join("app-data")));
    let opener = RecordingOpener::default();

    system.allow_user_path(&workspace).unwrap();
    system.reveal_path(&workspace_file, &opener).unwrap();

    assert_eq!(
        opener.calls(),
        vec![format!("reveal:{}", workspace_file.display())]
    );
}

#[test]
fn notifier_backend_receives_notification_request() {
    let temp = tempfile::tempdir().unwrap();
    let system = DesktopSystem::new(paths_with_app_data(temp.path().join("app-data")));
    let notifier = RecordingNotifier::default();

    system
        .notify("Generation finished", "job-1 completed", &notifier)
        .unwrap();

    assert_eq!(
        notifier.calls(),
        vec!["Generation finished|job-1 completed".to_owned()]
    );
}

fn paths_with_resource_dir(resource_dir: PathBuf) -> DesktopPaths {
    let root = resource_dir.parent().unwrap_or_else(|| Path::new("."));
    DesktopPaths {
        app_data_dir: root.join("data"),
        app_config_dir: root.join("config"),
        app_cache_dir: root.join("cache"),
        suggested_workspace_dir: root.join("workspace"),
        resource_dir: Some(resource_dir),
    }
}

fn paths_with_app_data(app_data_dir: PathBuf) -> DesktopPaths {
    let root = app_data_dir
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    DesktopPaths {
        app_data_dir,
        app_config_dir: root.join("config"),
        app_cache_dir: root.join("cache"),
        suggested_workspace_dir: root.join("workspace"),
        resource_dir: Some(root.join("resources")),
    }
}

fn touch(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, []).unwrap();
    path.to_path_buf()
}

fn touch_dir(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    std::fs::create_dir_all(path).unwrap();
    path.to_path_buf()
}

#[derive(Default)]
struct RecordingOpener {
    calls: Arc<Mutex<Vec<String>>>,
}

impl RecordingOpener {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl DesktopPathOpener for RecordingOpener {
    fn open_path(
        &self,
        path: &Path,
    ) -> nai_atelier_adapter_desktop_system::DesktopSystemResult<()> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("open:{}", path.display()));
        Ok(())
    }

    fn reveal_path(
        &self,
        path: &Path,
    ) -> nai_atelier_adapter_desktop_system::DesktopSystemResult<()> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("reveal:{}", path.display()));
        Ok(())
    }
}

struct RecordingDialog {
    directory: Option<PathBuf>,
    files: Vec<PathBuf>,
}

impl DesktopFileDialog for RecordingDialog {
    fn pick_directory(
        &self,
    ) -> nai_atelier_adapter_desktop_system::DesktopSystemResult<Option<PathBuf>> {
        Ok(self.directory.clone())
    }

    fn pick_files(
        &self,
        _options: PickFilesOptions,
    ) -> nai_atelier_adapter_desktop_system::DesktopSystemResult<Vec<PathBuf>> {
        Ok(self.files.clone())
    }
}

#[derive(Default)]
struct RecordingNotifier {
    calls: Arc<Mutex<Vec<String>>>,
}

impl RecordingNotifier {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl DesktopNotifier for RecordingNotifier {
    fn notify(
        &self,
        title: &str,
        body: &str,
    ) -> nai_atelier_adapter_desktop_system::DesktopSystemResult<()> {
        self.calls.lock().unwrap().push(format!("{title}|{body}"));
        Ok(())
    }
}

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn set_env(name: &str, value: Option<&Path>) {
    unsafe {
        match value {
            Some(value) => std::env::set_var(name, value),
            None => std::env::remove_var(name),
        }
    }
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
