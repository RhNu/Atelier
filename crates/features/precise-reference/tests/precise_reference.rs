use std::collections::BTreeMap;

use atelier_generation::CharacterReferenceType;
use atelier_precise_reference::{
    PreciseReferenceImage, PreciseReferenceImageReader, PreciseReferenceInput,
    PreciseReferenceService,
};
use atelier_resource_catalog::{ResourceId, ResourceKind, ResourceRef};

#[derive(Default)]
struct FakeImageReader {
    images: BTreeMap<ResourceId, PreciseReferenceImage>,
}

impl FakeImageReader {
    fn with_image(mut self, reference: &ResourceRef, kind: ResourceKind, payload: &str) -> Self {
        self.images.insert(
            reference.id.clone(),
            PreciseReferenceImage {
                kind,
                payload: payload.to_owned(),
            },
        );
        self
    }
}

impl PreciseReferenceImageReader for FakeImageReader {
    fn read_precise_reference_image(
        &self,
        source: &ResourceRef,
    ) -> atelier_precise_reference::PreciseReferenceResult<PreciseReferenceImage> {
        self.images.get(&source.id).cloned().ok_or_else(|| {
            atelier_precise_reference::PreciseReferenceError::not_found("missing image")
        })
    }
}

#[test]
fn prepares_image_resource_as_generation_character_reference() {
    let source = ResourceRef::base(ResourceId::new("reference-1"));
    let reader = FakeImageReader::default().with_image(
        &source,
        ResourceKind::ReferenceImage,
        "data:image/png;base64,AQID",
    );
    let service = PreciseReferenceService::new(reader);

    let reference = service
        .prepare(&PreciseReferenceInput {
            source,
            reference_type: CharacterReferenceType::Style,
            fidelity: 0.4,
            strength: 0.6,
        })
        .expect("reference should prepare");

    assert_eq!(reference.image, "data:image/png;base64,AQID");
    assert_eq!(reference.reference_type, CharacterReferenceType::Style);
    assert_float_eq(reference.fidelity, 0.4);
    assert_float_eq(reference.strength, 0.6);
}

#[test]
fn rejects_missing_or_non_image_resource() {
    let missing = ResourceRef::base(ResourceId::new("missing"));
    let service = PreciseReferenceService::new(FakeImageReader::default());
    let error = service
        .prepare(&PreciseReferenceInput {
            source: missing,
            reference_type: CharacterReferenceType::Character,
            fidelity: 0.5,
            strength: 0.6,
        })
        .unwrap_err();
    assert_eq!(error.kind().as_str(), "not_found");

    let source = ResourceRef::base(ResourceId::new("encoding-1"));
    let reader = FakeImageReader::default().with_image(&source, ResourceKind::VibeEncoding, "AQID");
    let service = PreciseReferenceService::new(reader);
    let error = service
        .prepare(&PreciseReferenceInput {
            source,
            reference_type: CharacterReferenceType::CharacterAndStyle,
            fidelity: 0.5,
            strength: 0.6,
        })
        .unwrap_err();
    assert_eq!(error.kind().as_str(), "invalid_resource_kind");
}

fn assert_float_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}
