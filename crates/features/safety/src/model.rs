use atelier_image_analysis::{ImageAnalysisModelInfo, ImageRatingScores};
use atelier_resource_catalog::ResourceRef;

use crate::{SafetyError, SafetyResult};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ImageSafetyScore(f32);

impl ImageSafetyScore {
    /// Creates a normalized fused safety score.
    ///
    /// # Errors
    /// Returns an error when the score is not finite or is outside `0.0..=1.0`.
    pub fn new(value: f32) -> SafetyResult<Self> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(SafetyError::invalid_score(
                "image safety score must be between 0.0 and 1.0",
            ))
        }
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SafetyRiskBand {
    Low,
    Medium,
    High,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SafetyLabel {
    Safe,
    Sensitive,
    Hidden,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SafetyModelEvidence {
    pub model: ImageAnalysisModelInfo,
    pub ratings: ImageRatingScores,
    pub fused_score: ImageSafetyScore,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SafetyReviewOutcome {
    NotNeeded,
    Disabled,
    Completed(SafetyModelEvidence),
    Failed {
        model: ImageAnalysisModelInfo,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SafetyAssessment {
    pub resource: ResourceRef,
    pub auto_label: SafetyLabel,
    pub risk_band: SafetyRiskBand,
    pub policy_id: String,
    pub policy_version: String,
    pub primary: SafetyModelEvidence,
    pub review: SafetyReviewOutcome,
    pub assessed_at_ms: Option<u64>,
}

impl SafetyAssessment {
    #[must_use]
    pub const fn effective_label(&self, manual_override: Option<SafetyLabel>) -> SafetyLabel {
        match manual_override {
            Some(label) => label,
            None => self.auto_label,
        }
    }

    #[must_use]
    pub const fn with_assessed_at_ms(mut self, assessed_at_ms: u64) -> Self {
        self.assessed_at_ms = Some(assessed_at_ms);
        self
    }
}
