//! Gallery domain model and artifact-backed index rules.

mod error;
mod model;
mod ports;
mod service;

pub use error::{GalleryError, GalleryErrorKind, GalleryResult};
pub use model::{
    GalleryImageReference, GalleryItem, GalleryItemId, GalleryQuery, GallerySafetyOverride,
    GallerySafetyState, GallerySourceKind, ImageReferenceTarget,
};
pub use ports::GalleryIndex;
pub use service::GalleryService;
