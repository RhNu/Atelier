use atelier_app_api::workspace::WorkspaceStatusDto;
use atelier_workspace::WorkspaceRoot;

pub struct WorkspaceUseCases<'a> {
    pub(crate) root: &'a WorkspaceRoot,
    pub(crate) schema_version: &'a u32,
}

impl WorkspaceUseCases<'_> {
    #[must_use]
    pub fn status(&self) -> WorkspaceStatusDto {
        WorkspaceStatusDto {
            root: self.root.as_path().to_path_buf(),
            schema_version: *self.schema_version,
            locked: true,
        }
    }
}
