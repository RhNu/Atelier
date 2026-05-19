use thiserror::Error;

pub type ResourceResult<T> = Result<T, ResourceCatalogError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResourceCatalogErrorKind {
    Repository,
    BlobStore,
    VariantBuilder,
    NotFound,
    InvalidState,
}

impl std::fmt::Display for ResourceCatalogErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Repository => "repository",
            Self::BlobStore => "blob_store",
            Self::VariantBuilder => "variant_builder",
            Self::NotFound => "not_found",
            Self::InvalidState => "invalid_state",
        };
        f.write_str(value)
    }
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind}: {message}")]
pub struct ResourceCatalogError {
    pub kind: ResourceCatalogErrorKind,
    pub message: String,
}

impl ResourceCatalogError {
    #[must_use]
    pub fn new(kind: ResourceCatalogErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn repository(message: impl Into<String>) -> Self {
        Self::new(ResourceCatalogErrorKind::Repository, message)
    }

    #[must_use]
    pub fn blob_store(message: impl Into<String>) -> Self {
        Self::new(ResourceCatalogErrorKind::BlobStore, message)
    }

    #[must_use]
    pub fn variant_builder(message: impl Into<String>) -> Self {
        Self::new(ResourceCatalogErrorKind::VariantBuilder, message)
    }

    #[must_use]
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ResourceCatalogErrorKind::NotFound, message)
    }

    #[must_use]
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(ResourceCatalogErrorKind::InvalidState, message)
    }
}
