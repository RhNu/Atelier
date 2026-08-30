use std::path::{Path, PathBuf};

use crate::{WorkspaceError, WorkspaceResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRoot {
    path: PathBuf,
}

impl WorkspaceRoot {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn join_relative(&self, relative: &WorkspaceRelativePath) -> PathBuf {
        let mut path = self.path.clone();
        for segment in relative.segments() {
            path.push(segment);
        }
        path
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceRelativePath {
    value: String,
}

impl WorkspaceRelativePath {
    /// Creates a controlled workspace-relative path.
    ///
    /// # Errors
    /// Returns an error when the path is empty, absolute, drive-prefixed, or
    /// contains traversal, empty, current-directory, or NUL segments.
    pub fn new(value: impl AsRef<str>) -> WorkspaceResult<Self> {
        let raw = value.as_ref();
        validate_raw_path(raw)?;
        let normalized = raw.replace('\\', "/");
        validate_normalized_path(&normalized)?;
        Ok(Self { value: normalized })
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.value.split('/')
    }
}

fn validate_raw_path(raw: &str) -> WorkspaceResult<()> {
    if raw.is_empty() {
        return Err(WorkspaceError::invalid_path("path must not be empty"));
    }
    if raw.contains('\0') {
        return Err(WorkspaceError::invalid_path("path must not contain NUL"));
    }
    if raw.starts_with(['/', '\\']) || has_windows_drive_prefix(raw) {
        return Err(WorkspaceError::invalid_path("path must be relative"));
    }
    Ok(())
}

fn validate_normalized_path(normalized: &str) -> WorkspaceResult<()> {
    for segment in normalized.split('/') {
        match segment {
            "" => {
                return Err(WorkspaceError::invalid_path(
                    "path segment must not be empty",
                ));
            }
            "." => return Err(WorkspaceError::invalid_path("path segment must not be `.`")),
            ".." => {
                return Err(WorkspaceError::invalid_path(
                    "path must not traverse upward",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

const fn has_windows_drive_prefix(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic()
}
