pub const WORKSPACE_FORMAT: &str = "atelier-workspace";
pub const WORKSPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceManifest {
    pub format: String,
    pub schema_version: u32,
}

impl Default for WorkspaceManifest {
    fn default() -> Self {
        Self {
            format: WORKSPACE_FORMAT.to_owned(),
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
        if self.format == WORKSPACE_FORMAT && self.schema_version == WORKSPACE_SCHEMA_VERSION {
            Ok(self)
        } else {
            Err(crate::WorkspaceError::unsupported_schema(format!(
                "unsupported workspace schema `{}` version {}; expected `{WORKSPACE_FORMAT}` \
                 version {WORKSPACE_SCHEMA_VERSION}",
                self.format, self.schema_version
            )))
        }
    }
}
