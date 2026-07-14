use atelier_prompt_lexicon::{
    PromptLexicon, PromptLexiconError, PromptLexiconListQuery, PromptLexiconMatchField,
    PromptLexiconMatchRank,
};
use std::sync::Arc;

#[test]
fn catalog_exposes_category_tree_and_stats() {
    let lexicon = sample_lexicon();

    let catalog = lexicon.catalog();

    assert_eq!(catalog.stats.total_tags, 7);
    assert_eq!(catalog.stats.categorized_tags, 3);
    assert_eq!(catalog.stats.uncategorized_tags, 4);
    assert_eq!(catalog.stats.source_count, 1);
    assert_eq!(catalog.categories.len(), 2);
    assert_eq!(catalog.categories[0].name, "people");
    assert_eq!(catalog.categories[0].tag_count, 3);
    assert_eq!(catalog.categories[0].subcategory_count, 1);
    assert_eq!(catalog.categories[0].subcategories[0].name, "basic");
    assert_eq!(catalog.categories[0].subcategories[0].tag_count, 3);
}

#[test]
fn search_prefers_exact_matches_by_tag_then_primary_then_alias() {
    let lexicon = sample_lexicon();

    let result = lexicon.search("hero pose", 10);
    let tags = result
        .items
        .iter()
        .map(|item| item.tag.as_str())
        .collect::<Vec<_>>();

    assert_eq!(tags[..3], ["hero_pose", "mirror_tag", "weighted_alias"]);
    assert_eq!(result.items[0].match_field, PromptLexiconMatchField::Tag);
    assert_eq!(result.items[0].match_rank, PromptLexiconMatchRank::Exact);
    assert_eq!(
        result.items[1].match_field,
        PromptLexiconMatchField::PrimaryTranslation
    );
    assert_eq!(result.items[1].match_rank, PromptLexiconMatchRank::Exact);
    assert_eq!(result.items[2].match_field, PromptLexiconMatchField::Alias);
    assert_eq!(result.items[2].matched_translation, "hero pose");

    let normalized = lexicon.search("hero pose", 10);
    let underscored = lexicon.search("hero_pose", 10);
    assert_eq!(normalized.items.first(), underscored.items.first());
}

#[test]
fn search_orders_prefix_then_substring_then_weight_and_tag() {
    let lexicon = sample_lexicon();

    let result = lexicon.search("hero", 10);
    let tags = result
        .items
        .iter()
        .map(|item| item.tag.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        tags[..6],
        [
            "heroine",
            "hero_pose",
            "mirror_tag",
            "weighted_alias",
            "alias_stage",
            "tag_sub_hero",
        ]
    );
    assert_eq!(result.items[0].match_rank, PromptLexiconMatchRank::Prefix);
    assert_eq!(result.items[0].match_field, PromptLexiconMatchField::Tag);
    assert_eq!(
        result.items[2].match_field,
        PromptLexiconMatchField::PrimaryTranslation
    );
    assert_eq!(result.items[3].match_field, PromptLexiconMatchField::Alias);
}

#[test]
fn search_empty_query_returns_no_results() {
    let lexicon = sample_lexicon();

    let result = lexicon.search("   ", 10);

    assert_eq!(result.total, 0);
    assert!(result.items.is_empty());
}

#[test]
fn list_pages_browse_results_and_reuses_search_order_for_queries() {
    let lexicon = sample_lexicon();

    let browse = lexicon
        .list(&PromptLexiconListQuery {
            query: String::new(),
            category: Some("people".to_owned()),
            subcategory: Some("basic".to_owned()),
            limit: 2,
            offset: 1,
        })
        .unwrap();
    let browse_tags = browse
        .items
        .iter()
        .map(|item| item.tag.as_str())
        .collect::<Vec<_>>();
    assert_eq!(browse.total, 3);
    assert_eq!(browse.offset, 1);
    assert_eq!(browse.limit, 2);
    assert_eq!(browse_tags, ["mirror_tag", "alias_stage"]);

    let search = lexicon
        .list(&PromptLexiconListQuery {
            query: "hero".to_owned(),
            category: None,
            subcategory: None,
            limit: 2,
            offset: 1,
        })
        .unwrap();
    let search_tags = search
        .items
        .iter()
        .map(|item| item.tag.as_str())
        .collect::<Vec<_>>();
    assert_eq!(search.total, 6);
    assert_eq!(search_tags, ["hero_pose", "mirror_tag"]);
}

#[test]
fn list_rejects_missing_browse_scope_when_query_is_empty() {
    let lexicon = sample_lexicon();

    let error = lexicon
        .list(&PromptLexiconListQuery {
            query: String::new(),
            category: None,
            subcategory: None,
            limit: 20,
            offset: 0,
        })
        .unwrap_err();

    assert!(matches!(error, PromptLexiconError::InvalidRequest(_)));
}

#[test]
fn rejects_unknown_schema_or_version() {
    let error = PromptLexicon::from_json_str(
        r#"{
            "schema": "prompt-lexicon-v2",
            "version": 2,
            "sources": [],
            "categories": [],
            "subcategories": [],
            "tags": [],
            "translations": [],
            "stats": {
                "total_tags": 0,
                "categorized_tags": 0,
                "uncategorized_tags": 0,
                "matched_weights": 0,
                "total_translations": 0,
                "tags_with_aliases": 0,
                "max_aliases_per_tag": 0,
                "source_count": 0,
                "manifest_version": 1,
                "primary_from_category_json": 0,
                "primary_from_manifest_sources": 0,
                "primary_fallback_to_tag": 0
            }
        }"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        PromptLexiconError::UnsupportedSchema { .. }
    ));
}

#[test]
fn rejects_out_of_range_payload_without_stats_without_panicking() {
    let error = PromptLexicon::from_json_str(
        r#"{
            "schema": "atelier-prompt-lexicon",
            "version": 1,
            "sources": [],
            "categories": [
                {
                    "name": "other",
                    "subcategory_start": 18446744073709551615,
                    "subcategory_count": 1
                }
            ],
            "subcategories": [],
            "tags": [],
            "translations": []
        }"#,
    )
    .unwrap_err();

    assert!(matches!(error, PromptLexiconError::InvalidPayload(_)));
}

#[test]
fn embedded_generated_asset_loads() {
    let lexicon = PromptLexicon::load_embedded_shared().unwrap();

    let catalog = lexicon.catalog();

    assert_eq!(catalog.stats.total_tags, 228_304);
    assert_eq!(catalog.stats.source_count, 6);
    assert_eq!(catalog.categories.len(), 16);

    let reused = PromptLexicon::load_embedded_shared().unwrap();
    assert!(Arc::ptr_eq(&lexicon, &reused));
}

fn sample_lexicon() -> PromptLexicon {
    PromptLexicon::from_json_str(SAMPLE_LEXICON_JSON).expect("sample lexicon should load")
}

const SAMPLE_LEXICON_JSON: &str = r#"{
            "schema": "atelier-prompt-lexicon",
            "version": 1,
            "sources": [
                {
                    "id": "fixture",
                    "path": "fixture.csv",
                    "parser": "weighted_csv",
                    "priority": 1,
                    "alias_only": false,
                    "allow_primary": true
                }
            ],
            "categories": [
                {
                    "name": "people",
                    "subcategory_start": 0,
                    "subcategory_count": 1
                },
                {
                    "name": "other",
                    "subcategory_start": 1,
                    "subcategory_count": 1
                }
            ],
            "subcategories": [
                {
                    "name": "basic",
                    "category_index": 0,
                    "tag_start": 0,
                    "tag_count": 3
                },
                {
                    "name": "H",
                    "category_index": 1,
                    "tag_start": 3,
                    "tag_count": 4
                }
            ],
            "tags": [
                {
                    "tag": "hero_pose",
                    "weight": 300,
                    "translation_start": 0,
                    "translation_count": 2
                },
                {
                    "tag": "mirror_tag",
                    "weight": 200,
                    "translation_start": 2,
                    "translation_count": 1
                },
                {
                    "tag": "alias_stage",
                    "weight": 100,
                    "translation_start": 3,
                    "translation_count": 2
                },
                {
                    "tag": "heroine",
                    "weight": 500,
                    "translation_start": 5,
                    "translation_count": 1
                },
                {
                    "tag": "tag_sub_hero",
                    "weight": 250,
                    "translation_start": 6,
                    "translation_count": 1
                },
                {
                    "tag": "weighted_alias",
                    "weight": 450,
                    "translation_start": 7,
                    "translation_count": 2
                },
                {
                    "tag": "fallback_only",
                    "weight": 50,
                    "translation_start": 9,
                    "translation_count": 1
                }
            ],
            "translations": [
                "hero primary",
                "hero alias",
                "hero pose",
                "alias primary",
                "hero pose",
                "hero runner",
                "sub hero translation",
                "weighted primary",
                "hero pose",
                "fallback_only"
            ],
            "stats": {
                "total_tags": 7,
                "categorized_tags": 3,
                "uncategorized_tags": 4,
                "matched_weights": 7,
                "total_translations": 10,
                "tags_with_aliases": 3,
                "max_aliases_per_tag": 1,
                "source_count": 1,
                "manifest_version": 1,
                "primary_from_category_json": 0,
                "primary_from_manifest_sources": 7,
                "primary_fallback_to_tag": 0
            }
        }"#;
