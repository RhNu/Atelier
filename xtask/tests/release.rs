use std::fs;
use std::path::Path;

use tempfile::tempdir;
use xtask::{prepare_app_release, validate_resource_catalog};

#[test]
fn release_prepare_updates_only_the_desktop_version_and_requires_new_stable_semver() {
    let directory = tempdir().unwrap();
    let package = directory.path().join("apps/desktop/package.json");
    fs::create_dir_all(package.parent().unwrap()).unwrap();
    let original = "{\n  \"name\": \"@atelier/desktop\",\n  \"version\": \"0.5.0\"\n}\n";
    fs::write(&package, original).unwrap();
    for invalid in ["0.5.0", "0.4.0", "0.6.0-beta.1", "0.6.0+build.1", "invalid"] {
        assert!(prepare_app_release(directory.path(), invalid).is_err());
        assert_eq!(fs::read_to_string(&package).unwrap(), original);
    }
    prepare_app_release(directory.path(), "0.6.0").unwrap();
    assert_eq!(
        fs::read_to_string(package).unwrap(),
        original.replace("0.5.0", "0.6.0")
    );
}

#[test]
fn source_catalog_validation_does_not_require_git_or_local_lfs_payloads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let directory = tempdir().unwrap();
    let target = directory.path().join("resources/catalog/catalog-v1.json");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::copy(root.join("resources/catalog/catalog-v1.json"), &target).unwrap();
    validate_resource_catalog(directory.path()).unwrap();
    let mut invalid: serde_json::Value =
        serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    invalid["resources"][0]["dependencies"] = serde_json::json!(["missing"]);
    fs::write(target, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(validate_resource_catalog(directory.path()).is_err());
}
