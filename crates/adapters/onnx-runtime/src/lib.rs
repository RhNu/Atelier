//! Process-wide ONNX Runtime initialization shared by desktop model adapters.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrtRuntimeError {
    #[error("ONNX Runtime library is missing: {0}")]
    Missing(PathBuf),
    #[error("failed to resolve ONNX Runtime library {path}: {source}")]
    Resolve {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("ONNX Runtime initialization lock is unavailable")]
    Lock,
    #[error("ONNX Runtime is already initialized from {active}; cannot switch to {requested}")]
    Conflict { active: PathBuf, requested: PathBuf },
    #[error("ONNX Runtime was configured before the desktop host initialized it")]
    AlreadyConfigured,
    #[error("ONNX Runtime initialization raced unexpectedly")]
    Race,
    #[error("ONNX Runtime error: {0}")]
    Runtime(String),
}

#[derive(Debug)]
pub struct OrtRuntime {
    library_path: PathBuf,
}

static ORT_RUNTIME: OnceLock<OrtRuntime> = OnceLock::new();
static ORT_RUNTIME_INIT: Mutex<()> = Mutex::new(());

/// Initializes the process-global runtime from a host-selected dynamic library.
///
/// Repeated initialization with the same canonical path is idempotent.
///
/// # Errors
/// Returns an error for missing libraries, path conflicts, or runtime load failures.
pub fn initialize(
    runtime_library_path: impl AsRef<Path>,
) -> Result<&'static OrtRuntime, OrtRuntimeError> {
    let requested_path = canonicalize(runtime_library_path.as_ref())?;
    let _guard = ORT_RUNTIME_INIT.lock().map_err(|_| OrtRuntimeError::Lock)?;
    if let Some(runtime) = ORT_RUNTIME.get() {
        return runtime.for_path(&requested_path);
    }

    let committed = ort::init_from(&requested_path)
        .map_err(|error| OrtRuntimeError::Runtime(error.to_string()))?
        .commit();
    if !committed {
        return Err(OrtRuntimeError::AlreadyConfigured);
    }
    ORT_RUNTIME
        .set(OrtRuntime {
            library_path: requested_path,
        })
        .map_err(|_| OrtRuntimeError::Race)?;
    ORT_RUNTIME.get().ok_or(OrtRuntimeError::Race)
}

impl OrtRuntime {
    #[must_use]
    pub fn library_path(&self) -> &Path {
        &self.library_path
    }

    /// Confirms that an adapter uses the process-global runtime path.
    ///
    /// # Errors
    /// Returns an error if the requested library differs from the active one.
    pub fn for_path(
        &'static self,
        requested_path: &Path,
    ) -> Result<&'static Self, OrtRuntimeError> {
        let requested = canonicalize(requested_path)?;
        if self.library_path == requested {
            return Ok(self);
        }
        Err(OrtRuntimeError::Conflict {
            active: self.library_path.clone(),
            requested,
        })
    }
}

fn canonicalize(path: &Path) -> Result<PathBuf, OrtRuntimeError> {
    if !path.is_file() {
        return Err(OrtRuntimeError::Missing(path.to_path_buf()));
    }
    path.canonicalize()
        .map_err(|source| OrtRuntimeError::Resolve {
            path: path.to_path_buf(),
            source,
        })
}
