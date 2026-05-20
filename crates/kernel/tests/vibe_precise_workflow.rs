mod support;

use futures_executor::block_on;
use nai_atelier_generation::CharacterReferenceType;
use nai_atelier_kernel::{
    EnsureVibeEncoding, ExportVibeDocument, ImportEmbeddedPngVibeDocument, ImportVibeDocument,
    KernelRuntime,
};
use nai_atelier_precise_reference::PreciseReferenceInput;
use nai_atelier_resource_catalog::{ResourceId, ResourceKind, ResourceRef};
use nai_atelier_vibe::{
    VibeEncodeSettings, VibeEncodingRecord, VibeExportFormat, VibeId, VibeModel, VibeSourceIdentity,
};
use support::MemoryKernelPorts;

const ENCODING_PAYLOAD: &str = "AQID";
const ENCODING_PAYLOAD_SHA256: &str =
    "b70035bb783a47bf61ac3ff70b005308e167ee984365690e638c1481b8ca2936";

#[test]
fn ensure_vibe_encoding_reuses_cached_bucket_without_calling_novelai() {
    let source = VibeSourceIdentity::new_sha256("source-hash");
    let settings = VibeEncodeSettings::new(VibeModel::NaiDiffusion45Full, 0.7).unwrap();
    let cached = VibeEncodingRecord {
        vibe_id: VibeId::new("vibe-1"),
        source: source.clone(),
        settings: settings.clone(),
        resource: ResourceRef::base(ResourceId::new("cached-encoding")),
    };
    let ports = MemoryKernelPorts::default().with_cached_vibe_encoding(cached.clone());
    let runtime = KernelRuntime::new(ports.clone());

    let ensured = block_on(runtime.ensure_vibe_encoding(EnsureVibeEncoding {
        vibe_id: VibeId::new("vibe-1"),
        source,
        image: "data:image/png;base64,AQID".to_owned(),
        settings,
    }))
    .expect("cached vibe should resolve");

    assert!(!ensured.created);
    assert_eq!(ensured.record, cached);
    assert_eq!(ports.encode_vibe_call_count(), 0);
}

#[test]
fn ensure_vibe_encoding_encodes_and_registers_cache_miss() {
    let source = VibeSourceIdentity::new_sha256("source-hash");
    let settings = VibeEncodeSettings::new(VibeModel::NaiDiffusion4Full, 0.8).unwrap();
    let ports = MemoryKernelPorts::default().with_encoded_vibe_payload("encoded-vibe");
    let runtime = KernelRuntime::new(ports.clone());

    let ensured = block_on(runtime.ensure_vibe_encoding(EnsureVibeEncoding {
        vibe_id: VibeId::new("vibe-1"),
        source: source.clone(),
        image: "data:image/png;base64,AQID".to_owned(),
        settings: settings.clone(),
    }))
    .expect("cache miss should encode");

    assert!(ensured.created);
    assert_eq!(ensured.record.source, source);
    assert_eq!(ensured.record.settings, settings);
    assert_eq!(ports.encode_vibe_call_count(), 1);
    let resource = &ports.registered_resources()[ensured.record.resource.id.as_str()];
    assert_eq!(resource.kind, ResourceKind::VibeEncoding);
    assert_eq!(resource.bytes, b"encoded-vibe");
}

#[test]
fn ensure_vibe_encoding_uses_normalized_value_and_source_scoped_resource_id() {
    let settings = VibeEncodeSettings::new(VibeModel::NaiDiffusion45Full, 0.7004).unwrap();
    let ports = MemoryKernelPorts::default();
    let runtime = KernelRuntime::new(ports.clone());

    let first = block_on(runtime.ensure_vibe_encoding(EnsureVibeEncoding {
        vibe_id: VibeId::new("vibe-1"),
        source: VibeSourceIdentity::new_sha256("source-a"),
        image: "data:image/png;base64,AQID".to_owned(),
        settings: settings.clone(),
    }))
    .expect("first source should encode");
    let second = block_on(runtime.ensure_vibe_encoding(EnsureVibeEncoding {
        vibe_id: VibeId::new("vibe-1"),
        source: VibeSourceIdentity::new_sha256("source-b"),
        image: "data:image/png;base64,BAUG".to_owned(),
        settings,
    }))
    .expect("second source should encode separately");

    assert_eq!(
        ports.registered_resources()[first.record.resource.id.as_str()].bytes,
        b"encoded:v4-5full:0.7"
    );
    assert_ne!(first.record.resource, second.record.resource);
}

#[test]
fn imports_embedded_png_vibe_document_through_extractor_and_resources() {
    let ports = MemoryKernelPorts::default().with_embedded_vibe_document(&official_vibe("Style A"));
    let runtime = KernelRuntime::new(ports.clone());

    let imported = block_on(runtime.import_embedded_png_vibe_document(
        ImportEmbeddedPngVibeDocument {
            file_name: "image.png".to_owned(),
            png_bytes: vec![137, 80, 78, 71],
        },
    ))
    .expect("embedded vibe should import");

    assert_eq!(imported.entries.len(), 1);
    let entry = &imported.entries[0];
    assert_eq!(entry.summary.display_name, "Style A");
    let resources = ports.registered_resources();
    assert_eq!(
        resources[entry.resources.document.id.as_str()].kind,
        ResourceKind::VibeDocument
    );
    assert_eq!(
        resources[entry.resources.encodings[0].id.as_str()].kind,
        ResourceKind::VibeEncoding
    );
}

#[test]
fn exports_managed_vibe_from_document_resource() {
    let ports = MemoryKernelPorts::default();
    let runtime = KernelRuntime::new(ports);
    let imported = block_on(runtime.import_vibe_document(ImportVibeDocument {
        file_name: "style.naiv4vibe".to_owned(),
        content: official_vibe("Style A"),
    }))
    .expect("official vibe should import");

    let exported = block_on(runtime.export_vibe_document(ExportVibeDocument {
        vibe_ids: vec![imported.entries[0].summary.document_id.clone()],
        format: VibeExportFormat::Naiv4vibe,
    }))
    .expect("official vibe should export");

    assert_eq!(exported.document.file_extension, "naiv4vibe");
    assert!(exported.document.content.contains("novelai-vibe-transfer"));
    assert!(exported.document.content.contains("Style A"));
}

#[test]
fn prepare_precise_reference_resolves_resource_payload() {
    let source = ResourceRef::base(ResourceId::new("reference-image"));
    let ports = MemoryKernelPorts::default().with_precise_reference_image(
        &source,
        ResourceKind::ReferenceImage,
        "data:image/png;base64,AQID",
    );
    let runtime = KernelRuntime::new(ports);

    let reference = block_on(runtime.prepare_precise_reference(PreciseReferenceInput {
        source,
        reference_type: CharacterReferenceType::CharacterAndStyle,
        fidelity: 0.35,
        strength: 0.75,
    }))
    .expect("precise reference should prepare");

    assert_eq!(reference.image, "data:image/png;base64,AQID");
    assert_eq!(
        reference.reference_type,
        CharacterReferenceType::CharacterAndStyle
    );
    assert_float_eq(reference.fidelity, 0.35);
    assert_float_eq(reference.strength, 0.75);
}

fn assert_float_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}

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
