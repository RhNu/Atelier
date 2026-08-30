use std::fs;
use std::time::Duration;

use atelier_adapter_downloadable_resources_fs::FileSystemDownloadableResourceManager;
use atelier_downloadable_resources::DownloadableResourceManager;
use tempfile::tempdir;

#[tokio::test]
async fn leases_defer_deletion_until_every_consumer_releases_the_version() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("downloadable-resources");
    let version_root = root.join("sample").join("1.0.0");
    fs::create_dir_all(&version_root).unwrap();
    fs::write(version_root.join("resource.json"), "{}").unwrap();
    fs::write(
        root.join("state.json"),
        r#"{
          "format":"atelier.downloadable-resource-state",
          "schema_version":1,
          "active":{"sample":"1.0.0"}
        }"#,
    )
    .unwrap();
    let manager = FileSystemDownloadableResourceManager::new(&root, "", "").unwrap();
    let first = manager.resolve("sample").unwrap();
    let second = manager.resolve("sample").unwrap();

    manager.delete("sample").await.unwrap();
    assert!(version_root.exists());
    drop(first);
    assert!(version_root.exists());
    drop(second);
    assert!(!version_root.exists());
}

#[test]
fn onboarding_and_exact_legacy_cleanup_are_persisted() {
    let directory = tempdir().unwrap();
    let app_data = directory.path();
    let root = app_data.join("downloadable-resources");
    let legacy = app_data.join("models/image-analysis");
    let unrelated = app_data.join("models/keep-me");
    fs::create_dir_all(&legacy).unwrap();
    fs::create_dir_all(&unrelated).unwrap();
    fs::write(legacy.join("model.onnx"), b"old").unwrap();
    fs::write(unrelated.join("data.bin"), b"keep").unwrap();
    let manager = FileSystemDownloadableResourceManager::new(&root, "", "").unwrap();

    assert!(!manager.onboarding_complete().unwrap());
    manager.complete_onboarding().unwrap();
    manager.cleanup_legacy_image_analysis(app_data).unwrap();

    assert!(manager.onboarding_complete().unwrap());
    assert!(!legacy.exists());
    assert!(unrelated.join("data.bin").is_file());
    for _ in 0..20 {
        if fs::read_dir(app_data.join("models")).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("deleting")
        }) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("renamed legacy directory was not removed in the background");
}
