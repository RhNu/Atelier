//! Danbooru-aware prompt lexicon domain contracts.
//!
//! This crate owns models, validation, normalization, and the engine port. Concrete
//! `SQLite`, filesystem, and `ONNX` behavior belongs to adapters.

mod error;
mod model;
mod normalize;

pub use error::{LexiconError, LexiconResult};
pub use model::*;
pub use normalize::{canonical_comparison_key, normalized_search_text};

/// Product-level access to the built-in lexicon.
///
/// Implementations must be thread-safe because completion and workbench queries
/// can be issued concurrently by the desktop frontend.
pub trait LexiconEngine: Send + Sync {
    /// Reports bundle capabilities and available facets.
    ///
    /// # Errors
    /// Returns an error if lexical metadata cannot be inspected.
    fn bootstrap(&self) -> LexiconResult<LexiconBootstrap>;

    /// Returns compact lexical completion candidates without loading semantic assets.
    ///
    /// # Errors
    /// Returns an error for invalid limits or unavailable lexical data.
    fn complete(&self, query: &str, limit: usize) -> LexiconResult<Vec<LexiconSearchItem>>;

    /// Searches lexical or semantic views.
    ///
    /// # Errors
    /// Returns an error for invalid queries or unavailable search capabilities.
    fn search(&self, query: &LexiconSearchQuery) -> LexiconResult<LexiconSearchPage>;

    /// Returns complete details and precomputed relationships for one entity.
    ///
    /// # Errors
    /// Returns an error if the entity does not exist or metadata cannot be read.
    fn entity(&self, entity_id: u64) -> LexiconResult<LexiconEntityDetail>;

    /// Looks up canonical names in one bounded local operation.
    ///
    /// Missing names are omitted rather than treated as errors.
    ///
    /// # Errors
    /// Returns an error for oversized batches or unavailable lexical data.
    fn lookup_canonical_names(&self, names: &[String]) -> LexiconResult<Vec<LexiconSearchItem>>;

    /// Resolves a batch atomically for prompt insertion.
    ///
    /// # Errors
    /// Returns an error if any entity is invalid or metadata cannot be read.
    fn resolve_entities(&self, entity_ids: &[u64]) -> LexiconResult<Vec<ResolvedLexiconEntity>>;
}

/// Engine used when an optional built-in bundle is absent or invalid.
#[derive(Clone, Debug)]
pub struct UnavailableLexicon {
    message: String,
}

impl UnavailableLexicon {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Default for UnavailableLexicon {
    fn default() -> Self {
        Self::new("built-in lexicon bundle is unavailable")
    }
}

impl LexiconEngine for UnavailableLexicon {
    fn bootstrap(&self) -> LexiconResult<LexiconBootstrap> {
        Ok(LexiconBootstrap::unavailable(self.message.clone()))
    }

    fn complete(&self, _query: &str, _limit: usize) -> LexiconResult<Vec<LexiconSearchItem>> {
        Err(LexiconError::unavailable(self.message.clone()))
    }

    fn search(&self, _query: &LexiconSearchQuery) -> LexiconResult<LexiconSearchPage> {
        Err(LexiconError::unavailable(self.message.clone()))
    }

    fn entity(&self, _entity_id: u64) -> LexiconResult<LexiconEntityDetail> {
        Err(LexiconError::unavailable(self.message.clone()))
    }

    fn lookup_canonical_names(&self, _names: &[String]) -> LexiconResult<Vec<LexiconSearchItem>> {
        Err(LexiconError::unavailable(self.message.clone()))
    }

    fn resolve_entities(&self, _entity_ids: &[u64]) -> LexiconResult<Vec<ResolvedLexiconEntity>> {
        Err(LexiconError::unavailable(self.message.clone()))
    }
}
