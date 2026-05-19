#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum WorkspaceSlot {
    ManifestFile,
    LockFile,
    ResourceBlobs,
    ResourceStaging,
    ResourceVariants,
    Database,
    Cache,
    Exports,
}

impl WorkspaceSlot {
    #[must_use]
    pub const fn is_directory(self) -> bool {
        !matches!(self, Self::ManifestFile | Self::LockFile)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceLayout;

impl WorkspaceLayout {
    #[must_use]
    pub const fn directory_slots(self) -> &'static [WorkspaceSlot] {
        &[
            WorkspaceSlot::ResourceBlobs,
            WorkspaceSlot::ResourceStaging,
            WorkspaceSlot::ResourceVariants,
            WorkspaceSlot::Database,
            WorkspaceSlot::Cache,
            WorkspaceSlot::Exports,
        ]
    }
}
