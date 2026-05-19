//! Workspace feature crate.

mod error;
mod layout;
mod lock;
mod paths;
mod ports;
mod version;

pub use error::{WorkspaceError, WorkspaceErrorKind, WorkspaceResult};
pub use layout::{WorkspaceLayout, WorkspaceSlot};
pub use lock::{WorkspaceLockMetadata, WorkspaceLockRequest};
pub use paths::{WorkspaceRelativePath, WorkspaceRoot};
pub use ports::{WorkspaceLock, WorkspaceLockLease, WorkspaceStore};
pub use version::{WORKSPACE_SCHEMA_VERSION, WorkspaceManifest};
