use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::tempdir;
use xtask::{prepare_app_release, tag_app_release, validate_resource, validate_resource_tag};

#[test]
fn release_prepare_requires_a_newer_stable_semver_and_updates_only_the_desktop_package() {
    let directory = tempdir().unwrap();
    let package = directory.path().join("apps/desktop/package.json");
    fs::create_dir_all(package.parent().unwrap()).unwrap();
    fs::write(
        &package,
        "{\n  \"name\": \"@atelier/desktop\",\n  \"version\": \"0.5.0\"\n}\n",
    )
    .unwrap();

    assert!(prepare_app_release(directory.path(), "0.5.0").is_err());
    assert!(prepare_app_release(directory.path(), "0.6.0-beta.1").is_err());
    prepare_app_release(directory.path(), "0.6.0").unwrap();
    assert!(
        fs::read_to_string(package)
            .unwrap()
            .contains("\"version\": \"0.6.0\"")
    );
}

#[test]
fn release_tag_rejects_a_dirty_worktree() {
    let directory = tempdir().unwrap();
    run_git(directory.path(), &["init"]);
    fs::write(directory.path().join("untracked.txt"), "dirty").unwrap();
    assert_eq!(
        tag_app_release(directory.path()).unwrap_err(),
        "release tags require a clean working tree"
    );
}

#[test]
fn resource_tag_must_match_the_catalog_version() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert!(validate_resource_tag(root, "resource-anime-dbrating-v1.0.0").is_ok());
    assert!(validate_resource_tag(root, "resource-anime-dbrating-v2.0.0").is_err());
    assert!(validate_resource_tag(root, "not-a-resource-tag").is_err());
}

#[test]
fn repository_owned_resource_requires_real_lfs_payload_files() {
    let directory = tempdir().unwrap();
    let catalog = directory.path().join("resources/catalog/catalog-v1.json");
    fs::create_dir_all(catalog.parent().unwrap()).unwrap();
    fs::write(
        catalog,
        r#"{"resources":[{"id":"lexicon-core","version":"1.0.0","files":[{"path":"lexicon.sqlite","size_bytes":10,"sha256":"0000000000000000000000000000000000000000000000000000000000000000"}]}]}"#,
    )
    .unwrap();
    assert!(validate_resource(directory.path(), "lexicon-core").is_err());
}

fn run_git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}
