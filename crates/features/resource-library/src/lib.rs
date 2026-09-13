//! Logical organization for workspace-owned creative resources.
//!
//! This feature owns stable library identities, user-facing paths, and folder
//! tree rules. Feature crates continue to own resource contents and lifecycle.

mod error;
mod identifier;
mod model;
mod tree;

pub use error::{ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryResult};
pub use identifier::{ResourceIdentifier, ResourcePath};
pub use model::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    ResourceName,
};
pub use tree::{LibraryChildren, LibraryNodeRef, LibraryTree};
