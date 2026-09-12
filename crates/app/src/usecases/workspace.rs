use super::{WorkspaceSession, WorkspaceStatusDto};

pub struct WorkspaceUseCases<'a, S, F, E> {
    pub(crate) app: &'a WorkspaceSession<S, F, E>,
}

impl<S, F, E> WorkspaceUseCases<'_, S, F, E> {
    #[must_use]
    pub fn status(&self) -> WorkspaceStatusDto {
        WorkspaceStatusDto {
            root: self.app.root.as_path().to_path_buf(),
            schema_version: self.app.schema_version,
            locked: true,
        }
    }
}
