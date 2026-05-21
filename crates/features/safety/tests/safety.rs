use async_trait::async_trait;
use atelier_resource_catalog::{ResourceId, ResourceRef};
use atelier_safety::{
    ImageSafetyScore, SafetyAssessment, SafetyErrorKind, SafetyLabel, SafetyModelScore,
    SafetyRiskBand, SafetyScanInput, SafetyScanner,
};
use futures_executor::block_on;

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
fn scanned_assessment_preserves_model_scores_and_derives_safety_labels() {
    let resource = ResourceRef::base(ResourceId::new("resource-1"));
    let assessment = SafetyAssessment::from_model_scores(
        resource.clone(),
        vec![
            SafetyModelScore::new("safe", 0.09).unwrap(),
            SafetyModelScore::new("nsfw", 0.91).unwrap(),
        ],
    )
    .unwrap()
    .with_scorer("open_nsfw@onnx", Some("1"))
    .with_assessed_at_ms(123);

    assert_eq!(assessment.resource, resource);
    assert_score(assessment.score, 0.91);
    assert_eq!(
        assessment.safe_score.map(ImageSafetyScore::value),
        Some(0.09)
    );
    assert_eq!(assessment.raw_scores.len(), 2);
    assert_eq!(assessment.risk_band(), SafetyRiskBand::High);
    assert_eq!(assessment.auto_label(), SafetyLabel::Sensitive);
    assert_eq!(
        assessment.effective_label(Some(SafetyLabel::Hidden)),
        SafetyLabel::Hidden
    );
}

#[test]
fn safety_thresholds_keep_medium_scores_visible_by_default() {
    let resource = ResourceRef::base(ResourceId::new("resource-1"));
    let low = SafetyAssessment::new(resource.clone(), ImageSafetyScore::new(0.19).unwrap());
    let medium = SafetyAssessment::new(resource.clone(), ImageSafetyScore::new(0.20).unwrap());
    let high = SafetyAssessment::new(resource, ImageSafetyScore::new(0.80).unwrap());

    assert_eq!(low.risk_band(), SafetyRiskBand::Low);
    assert_eq!(low.auto_label(), SafetyLabel::Safe);
    assert_eq!(medium.risk_band(), SafetyRiskBand::Medium);
    assert_eq!(medium.auto_label(), SafetyLabel::Safe);
    assert_eq!(high.risk_band(), SafetyRiskBand::High);
    assert_eq!(high.auto_label(), SafetyLabel::Sensitive);
}

#[test]
fn fake_safety_scanner_returns_deterministic_score_without_io() {
    block_on(async {
        let scanner = FakeSafetyScanner {
            score: ImageSafetyScore::new(0.2).unwrap(),
        };
        let resource = ResourceRef::base(ResourceId::new("resource-1"));

        let assessment = scanner
            .scan_image(SafetyScanInput {
                resource: resource.clone(),
                bytes: vec![1, 2, 3],
                mime_type: Some("image/png".to_owned()),
            })
            .await
            .unwrap();

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
    async fn scan_image(
        &self,
        input: SafetyScanInput,
    ) -> atelier_safety::SafetyResult<SafetyAssessment> {
        Ok(SafetyAssessment::new(input.resource, self.score).with_scorer("fake", None))
    }
}
