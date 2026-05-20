use nai_atelier_vibe::{
    VibeDocumentCodec, VibeEncodeSettings, VibeExportFormat, VibeModel, VibeSourceIdentity,
};

const ENCODING_PAYLOAD: &str = "AQID";
const ENCODING_PAYLOAD_SHA256: &str =
    "b70035bb783a47bf61ac3ff70b005308e167ee984365690e638c1481b8ca2936";

fn official_vibe(name: &str) -> String {
    format!(
        r#"{{
  "identifier": "novelai-vibe-transfer",
  "version": 1,
  "type": "encoding",
  "id": "{ENCODING_PAYLOAD_SHA256}",
  "name": "{name}",
  "encodings": {{
    "v4-5full": {{
      "default": {{
        "encoding": "{ENCODING_PAYLOAD}",
        "params": {{ "information_extracted": 0.7 }}
      }}
    }}
  }}
}}"#
    )
}

#[test]
fn imports_official_single_vibe_document() {
    let imported = VibeDocumentCodec::import_text("style.naiv4vibe", &official_vibe("Style A"))
        .expect("official vibe should import");

    assert_eq!(imported.entries.len(), 1);
    let entry = &imported.entries[0];
    assert_eq!(entry.summary.display_name, "Style A");
    assert_eq!(entry.summary.document_id.as_str(), ENCODING_PAYLOAD_SHA256);
    assert_eq!(entry.summary.available_encoding_configs.len(), 1);
    assert_eq!(entry.encoding_payloads.len(), 1);
    assert_eq!(
        entry.summary.available_encoding_configs[0].model,
        VibeModel::NaiDiffusion45Full
    );
    assert_eq!(
        entry.summary.available_encoding_configs[0]
            .settings
            .information_extracted_key(),
        "0.700"
    );
}

#[test]
fn imports_official_vibe_bundle_as_multiple_entries() {
    let bundle = format!(
        r#"{{
  "identifier": "novelai-vibe-transfer-bundle",
  "version": 1,
  "vibes": [
    {},
    {}
  ]
}}"#,
        official_vibe("Style A"),
        official_vibe("Style B")
    );

    let imported = VibeDocumentCodec::import_text("styles.naiv4vibebundle", &bundle)
        .expect("official bundle should import");

    assert_eq!(imported.entries.len(), 2);
    assert_eq!(imported.entries[0].summary.display_name, "Style A");
    assert_eq!(imported.entries[1].summary.display_name, "Style B");
}

#[test]
fn rejects_invalid_official_vibe_documents() {
    let invalid_identifier =
        official_vibe("Bad").replace("novelai-vibe-transfer", "not-novelai-vibe-transfer");
    let error = VibeDocumentCodec::import_text("bad.naiv4vibe", &invalid_identifier).unwrap_err();
    assert_eq!(error.kind().as_str(), "invalid_document");

    let empty_bundle = r#"{
  "identifier": "novelai-vibe-transfer-bundle",
  "version": 1,
  "vibes": []
}"#;
    let error = VibeDocumentCodec::import_text("empty.naiv4vibebundle", empty_bundle).unwrap_err();
    assert_eq!(error.kind().as_str(), "invalid_document");

    let hash_mismatch = official_vibe("Bad").replace(ENCODING_PAYLOAD_SHA256, &"0".repeat(64));
    let error = VibeDocumentCodec::import_text("hash.naiv4vibe", &hash_mismatch).unwrap_err();
    assert_eq!(error.kind().as_str(), "invalid_document");

    let malformed_later_encoding = official_vibe("Bad").replace(
        r#""default": {
        "encoding": "AQID",
        "params": { "information_extracted": 0.7 }
      }"#,
        r#""default": {
        "encoding": "AQID",
        "params": { "information_extracted": 0.7 }
      },
      "broken": {
        "encoding": "@@@",
        "params": { "information_extracted": 0.7 }
      }"#,
    );
    let error = VibeDocumentCodec::import_text("broken-later.naiv4vibe", &malformed_later_encoding)
        .unwrap_err();
    assert_eq!(error.kind().as_str(), "invalid_document");
}

#[test]
fn exports_single_and_bundle_in_official_formats() {
    let single = VibeDocumentCodec::import_text("style.naiv4vibe", &official_vibe("Style A"))
        .expect("official vibe should import");
    let export_entries = single
        .entries
        .iter()
        .map(nai_atelier_vibe::VibeImportEntry::export_entry)
        .collect::<Vec<_>>();
    let single_export =
        VibeDocumentCodec::export_text(&export_entries, VibeExportFormat::Naiv4vibe)
            .expect("single vibe should export");

    assert_eq!(single_export.file_extension, "naiv4vibe");
    assert!(
        single_export
            .content
            .contains("\"identifier\": \"novelai-vibe-transfer\"")
    );
    assert!(
        !single_export
            .content
            .contains("novelai-vibe-transfer-bundle")
    );

    let bundle_export =
        VibeDocumentCodec::export_text(&export_entries, VibeExportFormat::Naiv4vibebundle)
            .expect("bundle should export");

    assert_eq!(bundle_export.file_extension, "naiv4vibebundle");
    assert!(
        bundle_export
            .content
            .contains("\"identifier\": \"novelai-vibe-transfer-bundle\"")
    );
    assert!(bundle_export.content.contains("\"vibes\""));
}

#[test]
fn encode_settings_build_stable_source_cache_key() {
    let source = VibeSourceIdentity::new_sha256("abcdef");
    let settings_a = VibeEncodeSettings::new(VibeModel::NaiDiffusion45Full, 0.7)
        .expect("settings should validate");
    let settings_b = VibeEncodeSettings::new(VibeModel::NaiDiffusion45Full, 0.7004)
        .expect("settings should validate");
    let different_model = VibeEncodeSettings::new(VibeModel::NaiDiffusion4Full, 0.7)
        .expect("settings should validate");

    assert_eq!(settings_a.information_extracted_key(), "0.700");
    assert_eq!(settings_a.cache_key(&source), settings_b.cache_key(&source));
    assert_ne!(
        settings_a.cache_key(&source),
        different_model.cache_key(&source)
    );
}
