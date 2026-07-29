use std::fs;

use atelier_adapter_lexicon_bundle::LexiconBundle;
use atelier_prompt_lexicon::{
    LexiconEngine, LexiconSearchFilters, LexiconSearchMode, LexiconSearchQuery,
};
use sha2::{Digest, Sha256};
use tempfile::{TempDir, tempdir};
use xtask::{LexiconBundleConfig, build_lexicon_bundle, validate_lexicon_bundle};

#[test]
fn builds_and_opens_deterministic_lexical_bundle() {
    let (temp, input, output) = fixture();
    let _keep_temp = temp;
    let summary = build_lexicon_bundle(&LexiconBundleConfig {
        input_dir: input,
        output_dir: output.clone(),
        bundle_version: "test-1".to_owned(),
    })
    .unwrap();
    assert_eq!(summary.entity_count, 2);
    assert!(!summary.semantic_available);

    let validation = validate_lexicon_bundle(&output).unwrap();
    assert_eq!(validation.entity_count, 2);
    let engine = LexiconBundle::open(output).unwrap();
    let results = engine
        .search(&LexiconSearchQuery {
            text: "初音".to_owned(),
            mode: LexiconSearchMode::Lexical,
            filters: LexiconSearchFilters::default(),
            selected_entity_ids: vec![],
            offset: 0,
            limit: 20,
        })
        .unwrap();
    assert_eq!(results.items[0].canonical_name, "hatsune_miku");
}

#[test]
fn validation_rejects_checksum_changes_and_lfs_pointers() {
    let (temp, input, output) = fixture();
    let _keep_temp = temp;
    build_lexicon_bundle(&LexiconBundleConfig {
        input_dir: input,
        output_dir: output.clone(),
        bundle_version: "test-1".to_owned(),
    })
    .unwrap();
    let mut database = fs::read(output.join("lexicon.sqlite")).unwrap();
    database[100] ^= 0xff;
    fs::write(output.join("lexicon.sqlite"), database).unwrap();
    assert!(
        validate_lexicon_bundle(&output)
            .unwrap_err()
            .contains("SHA-256")
    );

    fs::write(
        output.join("lexicon.sqlite"),
        "version https://git-lfs.github.com/spec/v1\n",
    )
    .unwrap();
    assert!(
        validate_lexicon_bundle(&output)
            .unwrap_err()
            .contains("git lfs pull")
    );
}

#[test]
fn semantic_vector_dimensions_are_validated_without_loading_onnx() {
    let (temp, input, output) = fixture();
    let _keep_temp = temp;
    let semantic = input.join("semantic");
    fs::create_dir_all(&semantic).unwrap();
    fs::write(semantic.join("model.onnx"), "fixture").unwrap();
    fs::write(semantic.join("tokenizer.json"), "{}").unwrap();
    fs::write(semantic.join("LICENSE-model.txt"), "MIT").unwrap();
    fs::write(semantic.join("identity.f16"), [0_u8; 8]).unwrap();
    fs::write(semantic.join("knowledge.f16"), [0_u8; 8]).unwrap();
    fs::write(
        semantic.join("config.json"),
        "{\"dimensions\":2,\"entity_count\":2}\n",
    )
    .unwrap();
    build_lexicon_bundle(&LexiconBundleConfig {
        input_dir: input,
        output_dir: output.clone(),
        bundle_version: "test-semantic".to_owned(),
    })
    .unwrap();
    fs::write(output.join("identity.f16"), [0_u8; 6]).unwrap();
    assert!(validate_lexicon_bundle(&output).is_err());
}

#[test]
fn enriched_entities_and_batch_provenance_are_bundled() {
    let (temp, input, output) = fixture();
    let _keep_temp = temp;
    install_enriched_fixture(&input);

    build_lexicon_bundle(&LexiconBundleConfig {
        input_dir: input,
        output_dir: output.clone(),
        bundle_version: "test-enriched".to_owned(),
    })
    .unwrap();
    let manifest = fs::read_to_string(output.join("manifest.json")).unwrap();
    assert!(manifest.contains("\"mode\": \"batch\""));
    assert!(manifest.contains("\"model\": \"test-model\""));
    let engine = LexiconBundle::open(output).unwrap();
    let results = engine
        .search(&LexiconSearchQuery {
            text: "电子歌姬".to_owned(),
            mode: LexiconSearchMode::Lexical,
            filters: LexiconSearchFilters::default(),
            selected_entity_ids: vec![],
            offset: 0,
            limit: 20,
        })
        .unwrap();
    assert_eq!(results.items[0].canonical_name, "hatsune_miku");
}

#[test]
fn enriched_entities_reject_semantic_vectors_without_matching_input_hash() {
    let (temp, input, output) = fixture();
    let _keep_temp = temp;
    install_enriched_fixture(&input);
    let semantic = input.join("semantic");
    fs::create_dir_all(&semantic).unwrap();
    fs::write(semantic.join("model.onnx"), "fixture").unwrap();
    fs::write(semantic.join("tokenizer.json"), "{}").unwrap();
    fs::write(semantic.join("LICENSE-model.txt"), "MIT").unwrap();
    fs::write(semantic.join("identity.f16"), [0_u8; 8]).unwrap();
    fs::write(semantic.join("knowledge.f16"), [0_u8; 8]).unwrap();
    fs::write(
        semantic.join("config.json"),
        "{\"dimensions\":2,\"entity_count\":2}\n",
    )
    .unwrap();

    let error = build_lexicon_bundle(&LexiconBundleConfig {
        input_dir: input,
        output_dir: output,
        bundle_version: "test-stale-semantic".to_owned(),
    })
    .unwrap_err();
    assert!(error.contains("does not bind vectors to enriched entities"));
}

fn install_enriched_fixture(input: &std::path::Path) {
    let base = input.join("entities.jsonl");
    let enriched = input.join("entities.enriched.jsonl");
    let enriched_text = fs::read_to_string(&base)
        .unwrap()
        .replace("初音未来", "电子歌姬");
    fs::write(&enriched, enriched_text).unwrap();
    fs::write(
        input.join("entities.enriched.provenance.json"),
        format!(
            concat!(
                "{{\"mode\":\"batch\",\"endpoint\":\"/v1/chat/completions\",",
                "\"model\":\"test-model\",\"prompt_hash\":\"{}\",",
                "\"entity_count\":2,\"input_sha256\":\"{}\",\"output_sha256\":\"{}\"}}\n"
            ),
            "a".repeat(64),
            sha256(&base),
            sha256(&enriched),
        ),
    )
    .unwrap();
}

fn sha256(path: &std::path::Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}

fn fixture() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    let output = temp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join("entities.jsonl"),
        concat!(
            "{\"id\":1,\"canonical_name\":\"1girl\",\"primary_translation\":\"单个女孩\",",
            "\"kind\":\"tag\",\"category\":\"general\",\"post_count\":1000,\"rating\":\"safe\",",
            "\"aliases\":[\"one_girl\"],\"translations\":[{\"locale\":\"zh-CN\",\"text\":\"单个女孩\"}],",
            "\"wiki\":[{\"locale\":\"en\",\"text\":\"One female character\"}],\"groups\":[]}\n",
            "{\"id\":2,\"canonical_name\":\"hatsune_miku\",\"primary_translation\":\"初音未来\",",
            "\"kind\":\"tag\",\"category\":\"character\",\"post_count\":500,\"rating\":\"safe\",",
            "\"aliases\":[\"miku\"],\"translations\":[{\"locale\":\"zh-CN\",\"text\":\"初音未来\"}],",
            "\"wiki\":[],\"groups\":[\"tag_group:vocaloid\"]}\n"
        ),
    )
    .unwrap();
    fs::write(
        input.join("groups.json"),
        "[{\"id\":\"tag_group:vocaloid\",\"name\":\"Vocaloid\",\"members\":[2]}]\n",
    )
    .unwrap();
    fs::write(
        input.join("relations.jsonl"),
        "{\"source_entity_id\":1,\"target_entity_id\":2,\"relation\":\"npmi\",\"npmi\":0.7}\n",
    )
    .unwrap();
    fs::write(input.join("provenance.json"), "{\"sources\":[]}\n").unwrap();

    (temp, input, output)
}
