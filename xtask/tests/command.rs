use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::run_in_workspace;

#[test]
fn succeeds_when_only_warning_threshold_is_exceeded() {
    let workspace = TestWorkspace::new("command_warning_only");
    workspace.write_file("crates/demo/src/lib.rs", "one\ntwo\nthree\nfour\n");

    let result = run_in_workspace(
        args([
            "xtask",
            "line-budget",
            "--warn-lines",
            "3",
            "--deny-lines",
            "6",
        ]),
        workspace.path(),
    );

    assert!(result.is_ok());
}

#[test]
fn fails_when_deny_threshold_is_exceeded() {
    let workspace = TestWorkspace::new("command_violation");
    workspace.write_file(
        "crates/demo/src/lib.rs",
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\n",
    );

    let result = run_in_workspace(
        args([
            "xtask",
            "line-budget",
            "--warn-lines",
            "3",
            "--deny-lines",
            "6",
        ]),
        workspace.path(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("line budget check failed"));
}

#[test]
fn rejects_warning_threshold_above_deny_threshold() {
    let workspace = TestWorkspace::new("command_invalid_threshold");
    let result = run_in_workspace(
        args([
            "xtask",
            "line-budget",
            "--warn-lines",
            "7",
            "--deny-lines",
            "6",
        ]),
        workspace.path(),
    );

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("warn-lines must be less than or equal to deny-lines")
    );
}

#[test]
fn release_prepare_legacy_command_remains_available() {
    let workspace = TestWorkspace::new("release_prepare");
    workspace.write_file(
        "apps/desktop/package.json",
        "{\n  \"name\": \"@atelier/desktop\",\n  \"version\": \"0.5.7\"\n}\n",
    );
    let result = run_in_workspace(
        args(["xtask", "release", "prepare", "0.5.8"]),
        workspace.path(),
    );
    assert!(result.is_ok());
    assert!(
        fs::read_to_string(workspace.path().join("apps/desktop/package.json"))
            .unwrap()
            .contains("\"version\": \"0.5.8\"")
    );
}

#[test]
fn release_requires_a_selector() {
    let workspace = TestWorkspace::new("release_selector");
    let error = run_in_workspace(args(["xtask", "release"]), workspace.path()).unwrap_err();
    assert!(error.contains("release requires VERSION, patch, minor, or major"));
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

    fn write_file(&self, relative_path: &str, contents: &str) {
        let path = self.path.join(relative_path);
        fs::create_dir_all(path.parent().expect("test file should have a parent"))
            .expect("test file parent should be created");
        fs::write(path, contents).expect("test file should be written");
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("test workspace should be removed");
    }
}
