//! HTTPS catalog and filesystem storage for global downloadable resources.

mod catalog;
mod download;
mod manager;
mod state;

pub use manager::FileSystemDownloadableResourceManager;
