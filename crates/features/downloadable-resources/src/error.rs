use thiserror::Error;

pub type DownloadableResourceResult<T> = Result<T, DownloadableResourceError>;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DownloadableResourceError {
    #[error("invalid resource catalog: {0}")]
    InvalidCatalog(String),
    #[error("downloadable resource is unavailable: {0}")]
    Unavailable(String),
    #[error("downloadable resource operation failed: {0}")]
    Operation(String),
    #[error("downloadable resource operation was cancelled")]
    Cancelled,
}
