use std::sync::{Arc, Mutex};

use atelier_downloadable_resources::{DownloadableResourceManager, InstalledResource};
use atelier_prompt_lexicon::{
    LexiconBootstrap, LexiconEngine, LexiconResult, LexiconSearchItem, LexiconSearchPage,
    LexiconSearchQuery, ResolvedLexiconEntity,
};

use crate::LexiconBundle;

pub struct ManagedLexiconBundle {
    resources: Arc<dyn DownloadableResourceManager>,
    current: Mutex<Option<LoadedBundle>>,
}

struct LoadedBundle {
    key: String,
    engine: Arc<LexiconBundle>,
    _core: InstalledResource,
    _semantic: Option<InstalledResource>,
}

impl ManagedLexiconBundle {
    #[must_use]
    pub fn new(resources: Arc<dyn DownloadableResourceManager>) -> Arc<Self> {
        Arc::new(Self {
            resources,
            current: Mutex::new(None),
        })
    }

    fn engine(&self) -> LexiconResult<Arc<LexiconBundle>> {
        let core = self.resources.resolve("lexicon-core").map_err(|error| {
            atelier_prompt_lexicon::LexiconError::Unavailable(error.to_string())
        })?;
        let semantic = self.resources.resolve("lexicon-semantic").ok();
        let key = format!(
            "{}:{}",
            core.version,
            semantic
                .as_ref()
                .map_or("none", |value| value.version.as_str())
        );
        let mut current = self.current.lock().map_err(|_| {
            atelier_prompt_lexicon::LexiconError::Unavailable(
                "managed lexicon state is unavailable".to_owned(),
            )
        })?;
        if let Some(engine) = current
            .as_ref()
            .filter(|loaded| loaded.key == key)
            .map(|loaded| loaded.engine.clone())
        {
            drop(current);
            return Ok(engine);
        }
        let engine = if let Some(semantic) = &semantic {
            LexiconBundle::open_with_roots(&core.root, &semantic.root)?
        } else {
            LexiconBundle::open_core(&core.root)?
        };
        *current = Some(LoadedBundle {
            key,
            engine: engine.clone(),
            _core: core,
            _semantic: semantic,
        });
        drop(current);
        Ok(engine)
    }
}

impl LexiconEngine for ManagedLexiconBundle {
    fn bootstrap(&self) -> LexiconResult<LexiconBootstrap> {
        self.engine()?.bootstrap()
    }
    fn complete(&self, query: &str, limit: usize) -> LexiconResult<Vec<LexiconSearchItem>> {
        self.engine()?.complete(query, limit)
    }
    fn search(&self, query: &LexiconSearchQuery) -> LexiconResult<LexiconSearchPage> {
        self.engine()?.search(query)
    }
    fn entity(&self, entity_id: u64) -> LexiconResult<atelier_prompt_lexicon::LexiconEntityDetail> {
        self.engine()?.entity(entity_id)
    }
    fn lookup_canonical_names(&self, names: &[String]) -> LexiconResult<Vec<LexiconSearchItem>> {
        self.engine()?.lookup_canonical_names(names)
    }
    fn resolve_entities(&self, entity_ids: &[u64]) -> LexiconResult<Vec<ResolvedLexiconEntity>> {
        self.engine()?.resolve_entities(entity_ids)
    }
}
