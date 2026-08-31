use std::fs;
use std::path::{Path, PathBuf};

use atelier_adapter_lexicon_bundle::{LexiconBundle, LexiconBundleManifest};
use atelier_prompt_lexicon::{
    LexiconEngine, LexiconSearchFilters, LexiconSearchMode, LexiconSearchQuery,
};
use tempfile::TempDir;
use xtask::{LexiconBundleConfig, build_lexicon_bundle};

#[test]
fn license_line_endings_are_equivalent_but_content_changes_are_rejected() {
    let (_temp, root) = semantic_fixture();
    assert_eq!(
        fs::read_to_string(root.join("LICENSE-model.txt")).unwrap(),
        "MIT\nCopyright test\n"
    );
    let manifest_path = root.join("manifest.json");
    let original: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    for expected in ["MIT\nCopyright test\n", "MIT\r\nCopyright test\r\n"] {
        let mut manifest = original.clone();
        fs::write(root.join("LICENSE-model.txt"), expected).unwrap();
        manifest["semantic"]["license"]["sha256"] =
            super::sha256(&root.join("LICENSE-model.txt")).into();
        manifest["semantic"]["license"]["size_bytes"] = expected.len().into();
        fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        for actual in ["MIT\nCopyright test\n", "MIT\r\nCopyright test\r\n"] {
            fs::write(root.join("LICENSE-model.txt"), actual).unwrap();
            let engine = LexiconBundle::open(&root).unwrap();
            assert!(engine.bootstrap().unwrap().status.semantic_available);
        }
        fs::write(root.join("LICENSE-model.txt"), "MIT\nCopyright evil\n").unwrap();
        let engine = LexiconBundle::open(&root).unwrap();
        assert_lexical_available(engine.as_ref());
        assert!(
            engine
                .bootstrap()
                .unwrap()
                .status
                .message
                .unwrap()
                .contains("SHA-256")
        );
    }
}

#[test]
fn missing_or_truncated_semantic_assets_do_not_disable_lexical_operations() {
    for file in [
        "model.onnx",
        "tokenizer.json.zst",
        "LICENSE-model.txt",
        "identity.f16",
        "knowledge.f16",
    ] {
        let (_temp, root) = semantic_fixture();
        fs::remove_file(root.join(file)).unwrap();
        let engine = LexiconBundle::open_with_roots(&root, &root).unwrap();
        assert_lexical_available(engine.as_ref());
        assert!(!engine.bootstrap().unwrap().status.semantic_available);
        assert!(engine.search(&query(LexiconSearchMode::Semantic)).is_err());
        assert_lexical_available(engine.as_ref());
    }
    let (_temp, root) = semantic_fixture();
    fs::write(root.join("identity.f16"), [0_u8; 6]).unwrap();
    let engine = LexiconBundle::open(&root).unwrap();
    assert_lexical_available(engine.as_ref());
    assert!(!engine.bootstrap().unwrap().status.semantic_available);
}

#[test]
fn unsupported_semantic_metadata_does_not_prevent_core_parsing() {
    for (field, value) in [
        (
            "semantic",
            serde_json::json!({"tokenizer": {"encoding": "future-format"}}),
        ),
        ("ranking", serde_json::json!({"semantic_weight": "bad"})),
        ("ranking", serde_json::json!({"semantic_weight": 100})),
    ] {
        let (_temp, root) = semantic_fixture();
        let path = root.join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest[field] = value;
        fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert_lexical_available(LexiconBundle::open_core(&root).unwrap().as_ref());
        let engine = LexiconBundle::open(&root).unwrap();
        assert_lexical_available(engine.as_ref());
        assert!(!engine.bootstrap().unwrap().status.semantic_available);
        assert!(engine.bootstrap().unwrap().status.message.is_some());
    }
}

#[test]
fn lazy_semantic_failures_are_cached_without_poisoning_lexical_search() {
    for file in ["model.onnx", "identity.f16", "tokenizer.json.zst"] {
        let (_temp, root) = semantic_fixture();
        let path = root.join(file);
        let mut bytes = fs::read(&path).unwrap();
        bytes[0] ^= 0xff;
        fs::write(&path, bytes).unwrap();
        let engine = LexiconBundle::open(&root).unwrap();
        assert!(engine.bootstrap().unwrap().status.semantic_available);
        let first = engine
            .search(&query(LexiconSearchMode::Semantic))
            .unwrap_err();
        let second = engine
            .search(&query(LexiconSearchMode::Semantic))
            .unwrap_err();
        assert_eq!(first.to_string(), second.to_string());
        assert!(!engine.bootstrap().unwrap().status.semantic_available);
        assert!(engine.bootstrap().unwrap().status.message.is_some());
        assert_lexical_available(engine.as_ref());
    }
}

#[test]
fn tokenizer_identity_uses_decoded_content_and_enforces_limits() {
    let (_temp, root) = semantic_fixture();
    let manifest = LexiconBundleManifest::read(&root).unwrap();
    let mut tokenizer = manifest.semantic.unwrap().tokenizer;
    // Two zstd frames have different transport size/hash, but identical decoded bytes.
    let mut encoded = zstd::bulk::compress(b"{", 1).unwrap();
    encoded.extend(zstd::bulk::compress(b"}", 1).unwrap());
    fs::write(root.join(&tokenizer.bundle.file), encoded).unwrap();
    assert_eq!(tokenizer.decode(&root).unwrap(), b"{}");
    assert!(
        LexiconBundle::open(&root)
            .unwrap()
            .bootstrap()
            .unwrap()
            .status
            .semantic_available
    );
    fs::write(
        root.join(&tokenizer.bundle.file),
        zstd::bulk::compress(b"[]", 1).unwrap(),
    )
    .unwrap();
    assert!(
        tokenizer
            .decode(&root)
            .unwrap_err()
            .to_string()
            .contains("SHA-256")
    );
    fs::write(
        root.join(&tokenizer.bundle.file),
        zstd::bulk::compress(b"{} trailing", 1).unwrap(),
    )
    .unwrap();
    assert!(
        tokenizer
            .decode(&root)
            .unwrap_err()
            .to_string()
            .contains("size mismatch")
    );
    tokenizer.content_size_bytes = u64::MAX;
    assert!(tokenizer.decode(&root).is_err());
}

#[test]
fn corrupted_core_is_still_rejected() {
    let (_temp, root) = semantic_fixture();
    let path = root.join("lexicon.sqlite");
    let mut bytes = fs::read(&path).unwrap();
    bytes[100] ^= 0xff;
    fs::write(path, bytes).unwrap();
    assert!(LexiconBundle::open(&root).is_err());
}

fn assert_lexical_available(engine: &dyn LexiconEngine) {
    let bootstrap = engine.bootstrap().unwrap();
    assert!(bootstrap.status.lexical_available);
    assert_eq!(bootstrap.stats.total_entities, 2);
    assert_eq!(engine.complete("hatsune", 10).unwrap()[0].entity_id, 2);
    assert_eq!(
        engine
            .search(&query(LexiconSearchMode::Lexical))
            .unwrap()
            .items[0]
            .entity_id,
        2
    );
    assert_eq!(
        engine.entity(2).unwrap().entity.canonical_name,
        "hatsune_miku"
    );
    assert_eq!(
        engine
            .lookup_canonical_names(&["hatsune_miku".to_owned()])
            .unwrap()[0]
            .entity_id,
        2
    );
    assert_eq!(engine.resolve_entities(&[2]).unwrap()[0].entity_id, 2);
}

fn query(mode: LexiconSearchMode) -> LexiconSearchQuery {
    LexiconSearchQuery {
        text: "初音".to_owned(),
        mode,
        filters: LexiconSearchFilters::default(),
        selected_entity_ids: vec![],
        offset: 0,
        limit: 20,
    }
}

fn semantic_fixture() -> (TempDir, PathBuf) {
    let (temp, input, output) = super::fixture();
    install_semantic(&input);
    build_lexicon_bundle(&LexiconBundleConfig {
        input_dir: input,
        output_dir: output.clone(),
        bundle_version: "test-semantic".to_owned(),
    })
    .unwrap();
    (temp, output)
}

fn install_semantic(input: &Path) {
    let semantic = input.join("semantic");
    fs::create_dir_all(&semantic).unwrap();
    fs::write(semantic.join("model.onnx"), "fixture").unwrap();
    fs::write(semantic.join("tokenizer.json"), "{}").unwrap();
    fs::write(
        semantic.join("LICENSE-model.txt"),
        "MIT\r\nCopyright test\r\n",
    )
    .unwrap();
    fs::write(semantic.join("identity.f16"), [0_u8; 8]).unwrap();
    fs::write(semantic.join("knowledge.f16"), [0_u8; 8]).unwrap();
    fs::write(
        semantic.join("config.json"),
        "{\"dimensions\":2,\"entity_count\":2}\n",
    )
    .unwrap();
}
