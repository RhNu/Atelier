use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use async_trait::async_trait;
use atelier_image_analysis::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisInput, ImageAnalysisModelId,
    ImageAnalysisModelInfo, ImageAnalyzer,
};

use crate::{
    ImageSafetyScore, SafetyAssessment, SafetyError, SafetyLabel, SafetyModelEvidence,
    SafetyResult, SafetyReviewOutcome, SafetyRiskBand, SafetyScanInput, SafetyScanner,
    anime_rating_policy,
};

pub trait SafetyPolicyControl: Send + Sync {
    fn set_wd_auto_review_enabled(&self, enabled: bool);
    fn wd_auto_review_enabled(&self) -> bool;
}

pub struct SafetyPipeline {
    analyzer: Arc<dyn ImageAnalyzer>,
    wd_auto_review_enabled: AtomicBool,
}

impl SafetyPipeline {
    #[must_use]
    pub fn new(analyzer: Arc<dyn ImageAnalyzer>, wd_auto_review_enabled: bool) -> Self {
        Self {
            analyzer,
            wd_auto_review_enabled: AtomicBool::new(wd_auto_review_enabled),
        }
    }

    async fn analyze_ratings(
        &self,
        model: ImageAnalysisModelId,
        input: &SafetyScanInput,
    ) -> SafetyResult<ImageAnalysis> {
        self.analyzer
            .analyze(
                model,
                ImageAnalysisInput {
                    resource: input.resource.clone(),
                    bytes: input.bytes.clone(),
                    mime_type: input.mime_type.clone(),
                },
                AnalysisOutputSelection::ratings_only(),
            )
            .await
            .map_err(|error| SafetyError::scanner(error.to_string()))
    }
}

impl SafetyPolicyControl for SafetyPipeline {
    fn set_wd_auto_review_enabled(&self, enabled: bool) {
        self.wd_auto_review_enabled
            .store(enabled, Ordering::Release);
    }

    fn wd_auto_review_enabled(&self) -> bool {
        self.wd_auto_review_enabled.load(Ordering::Acquire)
    }
}

#[async_trait]
impl SafetyScanner for SafetyPipeline {
    async fn scan_image(&self, input: SafetyScanInput) -> SafetyResult<SafetyAssessment> {
        let wd_auto_review_enabled = self.wd_auto_review_enabled();
        let primary_analysis = self
            .analyze_ratings(ImageAnalysisModelId::AnimeDbRating, &input)
            .await?;
        let policy = anime_rating_policy();
        let primary = evidence(primary_analysis, |ratings| policy.primary_score(ratings))?;
        let primary_score = primary.fused_score.value();

        if primary_score >= policy.primary_threshold {
            return Ok(assessment(
                input.resource,
                SafetyLabel::Sensitive,
                SafetyRiskBand::High,
                primary,
                SafetyReviewOutcome::NotNeeded,
            ));
        }
        if !policy.should_review(primary_score) {
            return Ok(assessment(
                input.resource,
                SafetyLabel::Safe,
                SafetyRiskBand::Low,
                primary,
                SafetyReviewOutcome::NotNeeded,
            ));
        }
        if !wd_auto_review_enabled {
            return Ok(assessment(
                input.resource,
                SafetyLabel::Safe,
                SafetyRiskBand::Medium,
                primary,
                SafetyReviewOutcome::Disabled,
            ));
        }

        match self
            .analyze_ratings(ImageAnalysisModelId::WdSwinv2TaggerV3, &input)
            .await
        {
            Ok(review_analysis) => {
                let review = evidence(review_analysis, |ratings| policy.review_score(ratings))?;
                let sensitive = review.fused_score.value() >= policy.review_threshold;
                Ok(assessment(
                    input.resource,
                    if sensitive {
                        SafetyLabel::Sensitive
                    } else {
                        SafetyLabel::Safe
                    },
                    if sensitive {
                        SafetyRiskBand::High
                    } else {
                        SafetyRiskBand::Medium
                    },
                    primary,
                    SafetyReviewOutcome::Completed(review),
                ))
            }
            Err(error) => Ok(assessment(
                input.resource,
                SafetyLabel::Sensitive,
                SafetyRiskBand::High,
                primary,
                SafetyReviewOutcome::Failed {
                    model: ImageAnalysisModelInfo {
                        id: ImageAnalysisModelId::WdSwinv2TaggerV3,
                        revision: policy.review_model_revision.clone(),
                    },
                    message: error.to_string(),
                },
            )),
        }
    }
}

fn assessment(
    resource: atelier_resource_catalog::ResourceRef,
    auto_label: SafetyLabel,
    risk_band: SafetyRiskBand,
    primary: SafetyModelEvidence,
    review: SafetyReviewOutcome,
) -> SafetyAssessment {
    SafetyAssessment {
        resource,
        auto_label,
        risk_band,
        policy_id: anime_rating_policy().id.clone(),
        policy_version: anime_rating_policy().version.clone(),
        primary,
        review,
        assessed_at_ms: None,
    }
}

fn evidence(
    analysis: ImageAnalysis,
    score: impl FnOnce(atelier_image_analysis::ImageRatingScores) -> f32,
) -> SafetyResult<SafetyModelEvidence> {
    let ratings = analysis
        .ratings
        .ok_or_else(|| SafetyError::scanner("image analyzer did not return rating scores"))?;
    Ok(SafetyModelEvidence {
        model: analysis.model,
        ratings,
        fused_score: ImageSafetyScore::new(score(ratings))?,
    })
}
