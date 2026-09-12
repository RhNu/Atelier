use atelier_generation::CharacterReferenceType;
use atelier_precise_reference::{PreciseReferenceImage, prepare_reference};
use atelier_resource_catalog::ResourceKind;

#[test]
fn preserves_reference_payload_and_options() {
    let reference = prepare_reference(
        PreciseReferenceImage {
            kind: ResourceKind::ReferenceImage,
            payload: "data:image/png;base64,AQID".to_owned(),
        },
        CharacterReferenceType::Style,
        0.4,
        0.6,
    )
    .unwrap();
    assert_eq!(reference.image, "data:image/png;base64,AQID");
    assert_eq!(reference.reference_type, CharacterReferenceType::Style);
    assert!((reference.fidelity - 0.4).abs() < f32::EPSILON);
    assert!((reference.strength - 0.6).abs() < f32::EPSILON);
}

#[test]
fn rejects_non_image_and_empty_payloads() {
    for (kind, payload, code) in [
        (ResourceKind::VibeEncoding, "AQID", "invalid_resource_kind"),
        (ResourceKind::ReferenceImage, "  ", "empty_payload"),
    ] {
        let error = prepare_reference(
            PreciseReferenceImage {
                kind,
                payload: payload.to_owned(),
            },
            CharacterReferenceType::Character,
            0.5,
            0.6,
        )
        .unwrap_err();
        assert_eq!(error.kind().as_str(), code);
    }
}
