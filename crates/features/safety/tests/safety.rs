use std::sync::{Arc, Barrier};

use async_trait::async_trait;
use atelier_image_analysis::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisError, ImageAnalysisInput,
    ImageAnalysisModelId, ImageAnalysisModelInfo, ImageAnalysisResult, ImageAnalyzer,
    ImageRatingScores,
};
use atelier_resource_catalog::{ResourceId, ResourceRef};
use atelier_safety::{
    SafetyLabel, SafetyPipeline, SafetyPolicyControl, SafetyReviewOutcome, SafetyScanInput,
    SafetyScanner,
};

#[test]
fn default_pipeline_uses_primary_rating_model_without_review() {
    futures_executor::block_on(async {
        let pipeline = SafetyPipeline::new(Arc::new(FakeAnalyzer::default()), false);
        let assessment = pipeline.scan_image(input("safe")).await.unwrap();

        assert_eq!(assessment.auto_label, SafetyLabel::Safe);
        assert!(matches!(assessment.review, SafetyReviewOutcome::NotNeeded));
        assert_eq!(
            assessment.primary.model.id,
            ImageAnalysisModelId::AnimeDbRating
        );
    });
}

#[test]
fn enabled_review_promotes_borderline_primary_when_wd_is_explicit() {
    futures_executor::block_on(async {
        let pipeline = SafetyPipeline::new(
            Arc::new(FakeAnalyzer {
                primary: ImageRatingScores::new(0.0831, 0.2496, 0.2320, 0.4300).unwrap(),
                review: ImageRatingScores::new(0.0015, 0.0081, 0.0232, 0.9548).unwrap(),
                fail_review: false,
            }),
            true,
        );
        let assessment = pipeline.scan_image(input("borderline")).await.unwrap();

        assert_eq!(assessment.auto_label, SafetyLabel::Sensitive);
        assert!(matches!(
            assessment.review,
            SafetyReviewOutcome::Completed(_)
        ));
    });
}

#[test]
fn primary_sensitive_result_does_not_invoke_review() {
    futures_executor::block_on(async {
        let pipeline = SafetyPipeline::new(
            Arc::new(FakeAnalyzer {
                primary: ImageRatingScores::new(0.0, 0.0, 0.0, 1.0).unwrap(),
                ..FakeAnalyzer::default()
            }),
            true,
        );
        let assessment = pipeline.scan_image(input("sensitive")).await.unwrap();

        assert_eq!(assessment.auto_label, SafetyLabel::Sensitive);
        assert!(matches!(assessment.review, SafetyReviewOutcome::NotNeeded));
    });
}

#[test]
fn disabled_review_keeps_borderline_primary_safe() {
    futures_executor::block_on(async {
        let pipeline = SafetyPipeline::new(
            Arc::new(FakeAnalyzer {
                primary: borderline_primary(),
                ..FakeAnalyzer::default()
            }),
            false,
        );
        let assessment = pipeline.scan_image(input("disabled")).await.unwrap();

        assert_eq!(assessment.auto_label, SafetyLabel::Safe);
        assert!(matches!(assessment.review, SafetyReviewOutcome::Disabled));
    });
}

#[test]
fn enabled_review_can_release_borderline_primary() {
    futures_executor::block_on(async {
        let pipeline = SafetyPipeline::new(
            Arc::new(FakeAnalyzer {
                primary: borderline_primary(),
                review: ImageRatingScores::new(1.0, 0.0, 0.0, 0.0).unwrap(),
                fail_review: false,
            }),
            true,
        );
        let assessment = pipeline.scan_image(input("released")).await.unwrap();

        assert_eq!(assessment.auto_label, SafetyLabel::Safe);
        assert!(matches!(
            assessment.review,
            SafetyReviewOutcome::Completed(_)
        ));
    });
}

#[test]
fn review_failure_is_fail_closed_and_preserves_an_assessment() {
    futures_executor::block_on(async {
        let pipeline = SafetyPipeline::new(
            Arc::new(FakeAnalyzer {
                primary: ImageRatingScores::new(0.0831, 0.2496, 0.2320, 0.4300).unwrap(),
                review: ImageRatingScores::new(1.0, 0.0, 0.0, 0.0).unwrap(),
                fail_review: true,
            }),
            true,
        );
        let assessment = pipeline.scan_image(input("failure")).await.unwrap();

        assert_eq!(assessment.auto_label, SafetyLabel::Sensitive);
        assert!(matches!(
            assessment.review,
            SafetyReviewOutcome::Failed { .. }
        ));
    });
}

#[test]
fn policy_toggle_applies_to_subsequent_scans() {
    let pipeline = SafetyPipeline::new(Arc::new(FakeAnalyzer::default()), false);
    assert!(!pipeline.wd_auto_review_enabled());
    pipeline.set_wd_auto_review_enabled(true);
    assert!(pipeline.wd_auto_review_enabled());
}

#[test]
fn policy_toggle_does_not_change_an_in_flight_scan() {
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let pipeline = Arc::new(SafetyPipeline::new(
        Arc::new(BlockingAnalyzer {
            inner: FakeAnalyzer {
                primary: borderline_primary(),
                ..FakeAnalyzer::default()
            },
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        }),
        false,
    ));
    let scanning = Arc::clone(&pipeline);
    let handle = std::thread::spawn(move || {
        futures_executor::block_on(scanning.scan_image(input("in-flight"))).unwrap()
    });
    entered.wait();
    pipeline.set_wd_auto_review_enabled(true);
    release.wait();

    let assessment = handle.join().unwrap();
    assert!(matches!(assessment.review, SafetyReviewOutcome::Disabled));
    assert!(pipeline.wd_auto_review_enabled());
}

fn borderline_primary() -> ImageRatingScores {
    ImageRatingScores::new(0.0831, 0.2496, 0.2320, 0.4300).unwrap()
}

fn input(id: &str) -> SafetyScanInput {
    SafetyScanInput {
        resource: ResourceRef::base(ResourceId::new(format!("resource:{id}"))),
        bytes: vec![1, 2, 3],
        mime_type: Some("image/png".to_owned()),
    }
}

struct FakeAnalyzer {
    primary: ImageRatingScores,
    review: ImageRatingScores,
    fail_review: bool,
}

impl Default for FakeAnalyzer {
    fn default() -> Self {
        Self {
            primary: ImageRatingScores::new(0.99, 0.005, 0.003, 0.002).unwrap(),
            review: ImageRatingScores::new(0.99, 0.005, 0.003, 0.002).unwrap(),
            fail_review: false,
        }
    }
}

#[async_trait]
impl ImageAnalyzer for FakeAnalyzer {
    async fn analyze(
        &self,
        model: ImageAnalysisModelId,
        input: ImageAnalysisInput,
        outputs: AnalysisOutputSelection,
    ) -> ImageAnalysisResult<ImageAnalysis> {
        assert!(outputs.ratings);
        if model == ImageAnalysisModelId::WdSwinv2TaggerV3 && self.fail_review {
            return Err(ImageAnalysisError::inference("review failed"));
        }
        Ok(ImageAnalysis {
            resource: input.resource,
            model: ImageAnalysisModelInfo {
                id: model,
                revision: "test".to_owned(),
            },
            ratings: Some(if model == ImageAnalysisModelId::AnimeDbRating {
                self.primary
            } else {
                self.review
            }),
            general_tags: Vec::new(),
            character_tags: Vec::new(),
        })
    }
}

struct BlockingAnalyzer {
    inner: FakeAnalyzer,
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
}

#[async_trait]
impl ImageAnalyzer for BlockingAnalyzer {
    async fn analyze(
        &self,
        model: ImageAnalysisModelId,
        input: ImageAnalysisInput,
        outputs: AnalysisOutputSelection,
    ) -> ImageAnalysisResult<ImageAnalysis> {
        if model == ImageAnalysisModelId::AnimeDbRating {
            self.entered.wait();
            self.release.wait();
        }
        self.inner.analyze(model, input, outputs).await
    }
}
