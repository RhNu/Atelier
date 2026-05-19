use async_trait::async_trait;
use futures_executor::block_on;
use nai_atelier_resource_catalog::{ResourceId, ResourceRef};
use nai_atelier_safety::{ImageSafetyScore, SafetyAssessment, SafetyErrorKind, SafetyScanner};

#[test]
fn image_safety_score_accepts_range_boundaries() {
    assert_score(ImageSafetyScore::new(0.0).unwrap(), 0.0);
    assert_score(ImageSafetyScore::new(0.5).unwrap(), 0.5);
    assert_score(ImageSafetyScore::new(1.0).unwrap(), 1.0);
}

#[test]
fn image_safety_score_rejects_out_of_range_values() {
    let negative = ImageSafetyScore::new(-0.01).unwrap_err();
    let above_one = ImageSafetyScore::new(1.01).unwrap_err();
    let nan = ImageSafetyScore::new(f32::NAN).unwrap_err();

    assert_eq!(negative.kind(), SafetyErrorKind::InvalidScore);
    assert_eq!(above_one.kind(), SafetyErrorKind::InvalidScore);
    assert_eq!(nan.kind(), SafetyErrorKind::InvalidScore);
}

#[test]
fn safety_assessment_attaches_score_to_resource_ref() {
    let resource = ResourceRef::base(ResourceId::new("resource-1"));
    let assessment = SafetyAssessment::new(resource.clone(), ImageSafetyScore::new(0.75).unwrap())
        .with_scorer("fake-scorer", Some("0.1.0"))
        .with_assessed_at_ms(1_700_000_000_000);

    assert_eq!(assessment.resource, resource);
    assert_score(assessment.score, 0.75);
    assert_eq!(assessment.scorer_label.as_deref(), Some("fake-scorer"));
    assert_eq!(assessment.scorer_version.as_deref(), Some("0.1.0"));
    assert_eq!(assessment.assessed_at_ms, Some(1_700_000_000_000));
}

#[test]
fn fake_safety_scanner_returns_deterministic_score_without_io() {
    block_on(async {
        let scanner = FakeSafetyScanner {
            score: ImageSafetyScore::new(0.2).unwrap(),
        };
        let resource = ResourceRef::base(ResourceId::new("resource-1"));

        let assessment = scanner.score_image(resource.clone()).await.unwrap();

        assert_eq!(assessment.resource, resource);
        assert_score(assessment.score, 0.2);
        assert_eq!(assessment.scorer_label.as_deref(), Some("fake"));
    });
}

fn assert_score(score: ImageSafetyScore, expected: f32) {
    assert!((score.value() - expected).abs() < f32::EPSILON);
}

struct FakeSafetyScanner {
    score: ImageSafetyScore,
}

#[async_trait]
impl SafetyScanner for FakeSafetyScanner {
    async fn score_image(
        &self,
        resource: ResourceRef,
    ) -> nai_atelier_safety::SafetyResult<SafetyAssessment> {
        Ok(SafetyAssessment::new(resource, self.score).with_scorer("fake", None))
    }
}
