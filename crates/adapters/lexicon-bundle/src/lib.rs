//! Read-only `SQLite` and `ONNX` adapter for Atelier's built-in Danbooru lexicon.

mod managed;
mod manifest;
mod semantic;
mod sqlite;
mod text_assets;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use atelier_prompt_lexicon::{
    LexiconBootstrap, LexiconEngine, LexiconError, LexiconMatchReason, LexiconResult,
    LexiconSearchItem, LexiconSearchMode, LexiconSearchPage, LexiconSearchQuery,
    ResolvedLexiconEntity,
};

pub use managed::ManagedLexiconBundle;
pub use manifest::{
    BUNDLE_FORMAT, BUNDLE_SCHEMA_VERSION, BundleFile, DATABASE_SCHEMA_VERSION, EnrichmentManifest,
    LexiconBundleManifest, RankingManifest, SemanticManifest, SemanticModelContract,
    SourceManifest, TokenizerEncoding, TokenizerFile,
};

pub struct LexiconBundle {
    semantic_root: PathBuf,
    database_path: PathBuf,
    manifest: LexiconBundleManifest,
    semantic: OnceLock<Result<Mutex<semantic::SemanticEngine>, LexiconError>>,
    semantic_failure: OnceLock<LexiconError>,
}

impl LexiconBundle {
    /// Opens and structurally validates a built-in lexicon bundle.
    ///
    /// Invalid optional semantic assets are reported through capability status and never
    /// prevent lexical use. The ONNX session is initialized by the first semantic query.
    ///
    /// # Errors
    /// Returns an error when the manifest or lexical database is invalid.
    pub fn open(root: impl AsRef<Path>) -> LexiconResult<Arc<Self>> {
        Self::open_with_roots(root.as_ref(), root.as_ref())
    }

    /// Opens lexical and semantic data from separate downloadable-resource roots.
    ///
    /// # Errors
    /// Returns an error when the core manifest, database hash, or `SQLite` data is invalid.
    pub fn open_with_roots(core_root: &Path, semantic_root: &Path) -> LexiconResult<Arc<Self>> {
        let mut bundle = Self::load_core(core_root)?;
        bundle.semantic_root = semantic_root.to_path_buf();
        match LexiconBundleManifest::read_with_roots(core_root, semantic_root) {
            Ok(manifest) => bundle.manifest = manifest,
            Err(error) => {
                let _ = bundle.semantic.set(Err(error));
            }
        }
        Ok(Arc::new(bundle))
    }

    /// Opens only the lexical core resource.
    ///
    /// # Errors
    /// Returns an error when the manifest, hashes, or `SQLite` data are invalid.
    pub fn open_core(core_root: &Path) -> LexiconResult<Arc<Self>> {
        Self::load_core(core_root).map(Arc::new)
    }

    fn load_core(core_root: &Path) -> LexiconResult<Self> {
        let manifest = LexiconBundleManifest::read_core(core_root)?;
        manifest.verify_checksum(core_root, &manifest.database)?;
        let database_path = core_root.join(&manifest.database.file);
        let connection = sqlite::open_read_only(&database_path)?;
        sqlite::validate_database(&connection)?;
        drop(connection);
        Ok(Self {
            semantic_root: core_root.to_path_buf(),
            database_path,
            manifest,
            semantic: OnceLock::new(),
            semantic_failure: OnceLock::new(),
        })
    }

    fn connection(&self) -> LexiconResult<rusqlite::Connection> {
        sqlite::open_read_only(&self.database_path)
    }

    fn semantic_engine(&self) -> LexiconResult<&Mutex<semantic::SemanticEngine>> {
        if let Some(error) = self.semantic_failure.get() {
            return Err(error.clone());
        }
        self.semantic
            .get_or_init(|| {
                let manifest = self.manifest.semantic.as_ref().ok_or_else(|| {
                    LexiconError::SemanticUnavailable(
                        "bundle does not contain semantic assets".to_owned(),
                    )
                })?;
                manifest.verify_checksums(&self.semantic_root, &self.manifest)?;
                let connection = self.connection()?;
                let rows = sqlite::all_semantic_rows(&connection)?;
                semantic::SemanticEngine::load(
                    &self.semantic_root,
                    manifest,
                    self.manifest.ranking.clone(),
                    rows,
                )
                .map(Mutex::new)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn semantic_available(&self) -> bool {
        self.manifest.semantic.is_some() && self.semantic_error().is_none()
    }

    fn semantic_error(&self) -> Option<&LexiconError> {
        self.semantic_failure
            .get()
            .or_else(|| self.semantic.get().and_then(|result| result.as_ref().err()))
    }
}

impl LexiconEngine for LexiconBundle {
    fn bootstrap(&self) -> LexiconResult<LexiconBootstrap> {
        let connection = self.connection()?;
        Ok(LexiconBootstrap {
            bundle_version: Some(self.manifest.bundle_version.clone()),
            status: atelier_prompt_lexicon::LexiconCapabilityStatus {
                lexical_available: true,
                semantic_available: self.semantic_available(),
                message: self.semantic_error().map(ToString::to_string),
            },
            stats: sqlite::stats(&connection)?,
            categories: sqlite::category_facets(&connection)?,
            groups: sqlite::groups(&connection)?,
        })
    }

    fn complete(&self, query: &str, limit: usize) -> LexiconResult<Vec<LexiconSearchItem>> {
        if limit == 0 || limit > 50 {
            return Err(LexiconError::invalid_request(
                "completion limit must be between 1 and 50",
            ));
        }
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        sqlite::lexical_candidates(&self.connection()?, query, limit)
    }

    fn search(&self, query: &LexiconSearchQuery) -> LexiconResult<LexiconSearchPage> {
        query.validate()?;
        let connection = self.connection()?;
        let candidate_limit = query.offset.saturating_add(query.limit).clamp(200, 10_100);
        let (items, total) = match query.mode {
            LexiconSearchMode::Lexical => sqlite::lexical_search(&connection, query)?,
            LexiconSearchMode::Semantic => {
                if query.text.trim().is_empty() {
                    return Err(LexiconError::invalid_request(
                        "semantic search requires query text",
                    ));
                }
                let context = sqlite::context_scores(&connection, &query.selected_entity_ids)?;
                let engine = self.semantic_engine()?;
                let semantic = engine
                    .lock()
                    .map_err(|_| {
                        LexiconError::SemanticUnavailable(
                            "semantic engine lock is unavailable".to_owned(),
                        )
                    })
                    .and_then(|engine| {
                        engine.search(&query.text, &query.filters, &context, candidate_limit)
                    })
                    .inspect_err(|error| {
                        let _ = self.semantic_failure.set(error.clone());
                    })?;
                let semantic = sqlite::filter_items(&connection, semantic, &query.filters)?;
                let items = anchor_exact_matches(&connection, query, semantic)?;
                let total = items.len();
                let items = items
                    .into_iter()
                    .skip(query.offset)
                    .take(query.limit)
                    .collect();
                (items, total)
            }
        };
        Ok(LexiconSearchPage {
            items,
            total,
            offset: query.offset,
            limit: query.limit,
        })
    }

    fn entity(&self, entity_id: u64) -> LexiconResult<atelier_prompt_lexicon::LexiconEntityDetail> {
        sqlite::details(&self.connection()?, entity_id)
    }

    fn lookup_canonical_names(&self, names: &[String]) -> LexiconResult<Vec<LexiconSearchItem>> {
        sqlite::lookup_canonical_names(&self.connection()?, names)
    }

    fn resolve_entities(&self, entity_ids: &[u64]) -> LexiconResult<Vec<ResolvedLexiconEntity>> {
        sqlite::resolve_entities(&self.connection()?, entity_ids)
    }
}

fn anchor_exact_matches(
    connection: &rusqlite::Connection,
    query: &LexiconSearchQuery,
    semantic: Vec<LexiconSearchItem>,
) -> LexiconResult<Vec<LexiconSearchItem>> {
    let lexical = sqlite::lexical_candidates(connection, &query.text, 50)?;
    let anchors = sqlite::filter_items(connection, lexical, &query.filters)?
        .into_iter()
        .filter(|item| {
            matches!(
                item.match_reason,
                LexiconMatchReason::CanonicalExact
                    | LexiconMatchReason::AliasExact
                    | LexiconMatchReason::TranslationExact
            )
        })
        .collect::<Vec<_>>();
    let mut seen = HashSet::new();
    let mut results = Vec::with_capacity(anchors.len() + semantic.len());
    for item in anchors.into_iter().chain(semantic) {
        if seen.insert(item.entity_id) {
            results.push(item);
        }
    }
    Ok(results)
}
