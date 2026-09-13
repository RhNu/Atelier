//! Logical organization for workspace-owned creative resources.
//!
//! This feature owns stable library identities, user-facing paths, and folder
//! tree rules. Feature crates continue to own resource contents and lifecycle.

mod error;
mod identifier;
mod model;
mod ports;
mod service;
mod tree;

pub use error::{ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryResult};
pub use identifier::{ResourceIdentifier, ResourcePath};
pub use model::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    ResourceName,
};
pub use ports::ResourceLibraryRepository;
pub use service::{
    LibraryFolderEntry, LibraryResourceEntry, LibrarySnapshot, ResourceLibraryService,
    UpdateLibraryResourceRequest, UpsertLibraryFolderRequest,
};
pub use tree::{LibraryChildren, LibraryNodeRef, LibraryTree};
