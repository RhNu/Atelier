use nai_atelier_resource_catalog::ResourceRef;

use crate::{SafetyError, SafetyResult};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ImageSafetyScore(f32);

impl ImageSafetyScore {
    /// Creates a normalized image safety score.
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

#[derive(Clone, Debug, PartialEq)]
pub struct SafetyAssessment {
    pub resource: ResourceRef,
    pub score: ImageSafetyScore,
    pub scorer_label: Option<String>,
    pub scorer_version: Option<String>,
    pub assessed_at_ms: Option<u64>,
}

impl SafetyAssessment {
    #[must_use]
    pub const fn new(resource: ResourceRef, score: ImageSafetyScore) -> Self {
        Self {
            resource,
            score,
            scorer_label: None,
            scorer_version: None,
            assessed_at_ms: None,
        }
    }

    #[must_use]
    pub fn with_scorer(mut self, label: impl Into<String>, version: Option<&str>) -> Self {
        self.scorer_label = Some(label.into());
        self.scorer_version = version.map(str::to_owned);
        self
    }

    #[must_use]
    pub const fn with_assessed_at_ms(mut self, assessed_at_ms: u64) -> Self {
        self.assessed_at_ms = Some(assessed_at_ms);
        self
    }
}
