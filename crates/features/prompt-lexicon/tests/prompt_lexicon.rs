use atelier_prompt_lexicon::{
    LexiconEngine, LexiconSearchFilters, LexiconSearchMode, LexiconSearchQuery, UnavailableLexicon,
    canonical_comparison_key, normalized_search_text,
};

#[test]
fn comparison_keys_collapse_prompt_equivalent_spelling() {
    assert_eq!(
        canonical_comparison_key("  Cinematic Light  "),
        canonical_comparison_key("cinematic_light")
    );
    assert_eq!(normalized_search_text("红 色_头发"), "红 色 头发");
}

#[test]
fn search_query_rejects_unbounded_requests() {
    let query = LexiconSearchQuery {
        text: String::new(),
        mode: LexiconSearchMode::Lexical,
        filters: LexiconSearchFilters::default(),
        selected_entity_ids: Vec::new(),
        offset: 0,
        limit: 101,
    };

    assert!(query.validate().is_err());
}

#[test]
fn unavailable_engine_bootstraps_as_a_degraded_capability() {
    let engine = UnavailableLexicon::new("bundle checksum mismatch");
    let bootstrap = engine.bootstrap().expect("bootstrap remains available");

    assert!(!bootstrap.status.lexical_available);
    assert!(!bootstrap.status.semantic_available);
    assert_eq!(
        bootstrap.status.message.as_deref(),
        Some("bundle checksum mismatch")
    );
    assert!(engine.complete("girl", 20).is_err());
}
