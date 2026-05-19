pub const WORKSPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceManifest {
    pub schema_version: u32,
}

impl Default for WorkspaceManifest {
    fn default() -> Self {
        Self {
            schema_version: WORKSPACE_SCHEMA_VERSION,
        }
    }
}

impl WorkspaceManifest {
    /// Validates this manifest against the current schema version.
    ///
    /// # Errors
    /// Returns an error when the manifest version is not supported.
    pub fn validate(self) -> crate::WorkspaceResult<Self> {
        if self.schema_version == WORKSPACE_SCHEMA_VERSION {
            Ok(self)
        } else {
            Err(crate::WorkspaceError::unsupported_version(
                self.schema_version,
            ))
        }
    }
}
