use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xtask::{LineBudgetConfig, LineBudgetLevel, check_line_budget};

#[test]
fn accepts_rs_files_at_or_below_the_warning_threshold() {
    let workspace = TestWorkspace::new("line_budget_accepts_small_files");
    workspace.write_file("crates/demo/src/lib.rs", "one\ntwo\nthree\n");
    workspace.write_file("apps/desktop/src-tauri/src/lib.rs", "one\ntwo\n");

    let report = check_line_budget(workspace.path(), &test_config(3, 6)).unwrap();

    assert!(report.findings.is_empty());
}

#[test]
fn reports_warning_without_violation_above_warning_threshold() {
    let workspace = TestWorkspace::new("line_budget_warns");
    workspace.write_file("crates/demo/src/lib.rs", "one\ntwo\nthree\nfour\n");

    let report = check_line_budget(workspace.path(), &test_config(3, 6)).unwrap();

    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].level, LineBudgetLevel::Warning);
    assert_eq!(report.findings[0].line_count, 4);
    assert_eq!(report.findings[0].threshold, 3);
}

#[test]
fn reports_violation_above_deny_threshold() {
    let workspace = TestWorkspace::new("line_budget_denies");
    workspace.write_file(
        "xtask/src/lib.rs",
        "one\ntwo\nthree\nfour\nfive\nsix\nseven\n",
    );

    let report = check_line_budget(workspace.path(), &test_config(3, 6)).unwrap();

    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].level, LineBudgetLevel::Violation);
    assert_eq!(report.findings[0].line_count, 7);
    assert_eq!(report.findings[0].threshold, 6);
}

#[test]
fn ignores_non_rs_files_and_files_outside_scan_roots() {
    let workspace = TestWorkspace::new("line_budget_ignores");
    workspace.write_file("crates/demo/README.md", "one\ntwo\nthree\nfour\n");
    workspace.write_file("outside.rs", "one\ntwo\nthree\nfour\n");

    let report = check_line_budget(workspace.path(), &test_config(3, 6)).unwrap();

    assert!(report.findings.is_empty());
}

#[test]
fn accepts_missing_scan_roots() {
    let workspace = TestWorkspace::new("line_budget_missing_roots");

    let report = check_line_budget(workspace.path(), &test_config(3, 6)).unwrap();

    assert!(report.findings.is_empty());
}

#[test]
fn counts_final_line_without_trailing_newline() {
    let workspace = TestWorkspace::new("line_budget_no_trailing_newline");
    workspace.write_file("crates/demo/src/lib.rs", "one\ntwo\nthree\nfour");

    let report = check_line_budget(workspace.path(), &test_config(3, 6)).unwrap();

    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].line_count, 4);
}

fn test_config(warn_lines: usize, deny_lines: usize) -> LineBudgetConfig {
    LineBudgetConfig {
        scan_roots: vec!["crates".into(), "apps".into(), "xtask".into()],
        warn_lines,
        deny_lines,
    }
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
