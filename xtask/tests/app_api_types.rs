use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::{AppApiTypeExportConfig, export_app_api_types, run_in_workspace};

#[test]
fn app_api_types_command_writes_generated_index() {
    let workspace = TestWorkspace::new("app_api_types_command");

    let result = run_in_workspace(args(["xtask", "app-api", "types"]), workspace.path());

    assert!(result.is_ok(), "{result:?}");
    let generated_dir = workspace.path().join("apps/desktop/src/types/generated");
    assert!(generated_dir.join("index.ts").exists());
    assert!(
        fs::read_to_string(generated_dir.join("index.ts"))
            .expect("index should be readable")
            .contains("WorkspaceStatusDto"),
    );
}

#[test]
fn app_api_type_export_clears_stale_types() {
    let workspace = TestWorkspace::new("app_api_types_stale");
    let generated_dir = workspace.path().join("apps/desktop/src/types/generated");
    fs::create_dir_all(&generated_dir).expect("generated dir should be created");
    fs::write(
        generated_dir.join("StaleDto.ts"),
        "export type StaleDto = {};\n",
    )
    .expect("stale type should be written");

    export_app_api_types(&AppApiTypeExportConfig::default_for_workspace(
        workspace.path(),
    ))
    .expect("types should export");

    assert!(!generated_dir.join("StaleDto.ts").exists());
    assert!(generated_dir.join("ErrorEnvelopeDto.ts").exists());
}

#[test]
fn app_api_type_export_clears_nested_stale_types() {
    let workspace = TestWorkspace::new("app_api_types_nested_stale");
    let generated_dir = workspace.path().join("apps/desktop/src/types/generated");
    let stale_nested_dir = generated_dir.join("stale");
    fs::create_dir_all(&stale_nested_dir).expect("nested generated dir should be created");
    fs::write(
        stale_nested_dir.join("StaleNestedDto.ts"),
        "export type StaleNestedDto = {};\n",
    )
    .expect("nested stale type should be written");

    export_app_api_types(&AppApiTypeExportConfig::default_for_workspace(
        workspace.path(),
    ))
    .expect("types should export");

    assert!(!stale_nested_dir.exists());
    assert!(generated_dir.join("serde_json/JsonValue.ts").exists());
}

fn args(values: impl IntoIterator<Item = &'static str>) -> Vec<OsString> {
    values.into_iter().map(OsString::from).collect()
}

struct TestWorkspace {
    path: PathBuf,
}

impl TestWorkspace {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("atelier_xtask_{name}_{unique}"));
        fs::create_dir_all(&path).expect("test workspace should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("test workspace should be removed");
    }
}
