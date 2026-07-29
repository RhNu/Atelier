use std::env;
use std::time::Instant;

use atelier_adapter_lexicon_bundle::LexiconBundle;
use atelier_prompt_lexicon::{
    LexiconEngine, LexiconSearchFilters, LexiconSearchMode, LexiconSearchQuery,
};

#[test]
#[ignore = "loads the native ONNX Runtime and the release lexicon bundle"]
fn bundled_semantic_search_smoke() {
    let bundle_path =
        env::var_os("ATELIER_LEXICON_BUNDLE").expect("ATELIER_LEXICON_BUNDLE must be set");
    let runtime_path = env::var_os("ATELIER_ONNX_RUNTIME_LIBRARY")
        .expect("ATELIER_ONNX_RUNTIME_LIBRARY must be set");
    atelier_adapter_onnx_runtime::initialize(runtime_path).expect("runtime should initialize");
    let lexicon = LexiconBundle::open(bundle_path).expect("bundle should open");

    let started = Instant::now();
    let page = lexicon
        .search(&LexiconSearchQuery {
            text: "蓝色头发的动漫女孩".to_owned(),
            mode: LexiconSearchMode::Semantic,
            filters: LexiconSearchFilters::default(),
            selected_entity_ids: Vec::new(),
            offset: 0,
            limit: 10,
        })
        .expect("semantic search should succeed");

    assert!(!page.items.is_empty());
    let cold_elapsed = started.elapsed();
    let warmed = Instant::now();
    let warmed_page = lexicon
        .search(&LexiconSearchQuery {
            text: "dramatic cinematic lighting".to_owned(),
            mode: LexiconSearchMode::Semantic,
            filters: LexiconSearchFilters::default(),
            selected_entity_ids: Vec::new(),
            offset: 0,
            limit: 10,
        })
        .expect("warmed semantic search should succeed");
    assert!(!warmed_page.items.is_empty());
    eprintln!(
        "semantic smoke returned {} results in {:?}; warmed query in {:?}",
        page.items.len(),
        cold_elapsed,
        warmed.elapsed()
    );
}
