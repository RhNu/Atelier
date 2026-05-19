use nai_atelier_workspace::{
    WORKSPACE_SCHEMA_VERSION, WorkspaceLayout, WorkspaceManifest, WorkspaceRelativePath,
    WorkspaceRoot, WorkspaceSlot,
};

#[test]
fn relative_path_accepts_workspace_local_paths_and_normalizes_separators() {
    let path = WorkspaceRelativePath::new("resources\\blobs/a.png").unwrap();

    assert_eq!(path.as_str(), "resources/blobs/a.png");
}

#[test]
fn relative_path_rejects_escape_and_absolute_inputs() {
    let invalid_inputs = ["../x", "a/../x", "C:\\x", "/x", "", "a\0b"];

    for input in invalid_inputs {
        let error = WorkspaceRelativePath::new(input).unwrap_err();
        assert!(
            error.to_string().contains("invalid workspace path"),
            "unexpected error for {input:?}: {error}"
        );
    }
}

#[test]
fn layout_exposes_directory_slots_without_freezing_storage_paths() {
    let layout = WorkspaceLayout;

    assert_eq!(
        layout.directory_slots(),
        &[
            WorkspaceSlot::ResourceBlobs,
            WorkspaceSlot::ResourceStaging,
            WorkspaceSlot::ResourceVariants,
            WorkspaceSlot::Database,
            WorkspaceSlot::Cache,
            WorkspaceSlot::Exports,
        ]
    );
    assert!(!WorkspaceSlot::ManifestFile.is_directory());
    assert!(!WorkspaceSlot::LockFile.is_directory());
    assert!(WorkspaceSlot::ResourceBlobs.is_directory());
}

#[test]
fn workspace_root_joins_only_controlled_relative_paths() {
    let root = WorkspaceRoot::new("D:\\atelier");
    let child = root.join_relative(&WorkspaceRelativePath::new("resources/blobs").unwrap());

    assert!(child.ends_with("resources\\blobs") || child.ends_with("resources/blobs"));
}

#[test]
fn manifest_defaults_to_current_schema_version() {
    let manifest = WorkspaceManifest::default();

    assert_eq!(WORKSPACE_SCHEMA_VERSION, 1);
    assert_eq!(manifest.schema_version, WORKSPACE_SCHEMA_VERSION);
}
