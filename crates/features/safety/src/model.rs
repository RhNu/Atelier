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
pub struct SafetyModelScore {
    pub label: String,
    pub score: ImageSafetyScore,
}

impl SafetyModelScore {
    /// Creates one raw model score entry.
    ///
    /// # Errors
    /// Returns an error when the score is not finite or outside `0.0..=1.0`.
    pub fn new(label: impl Into<String>, score: f32) -> SafetyResult<Self> {
        Ok(Self {
            label: label.into(),
            score: ImageSafetyScore::new(score)?,
        })
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
pub struct SafetyAssessment {
    pub resource: ResourceRef,
    pub score: ImageSafetyScore,
    pub safe_score: Option<ImageSafetyScore>,
    pub raw_scores: Vec<SafetyModelScore>,
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
            safe_score: None,
            raw_scores: Vec::new(),
            scorer_label: None,
            scorer_version: None,
            assessed_at_ms: None,
        }
    }

    /// Builds a scanned assessment from raw model outputs.
    ///
    /// The canonical NSFW score is taken from the score labeled `nsfw`.
    /// # Errors
    /// Returns an error when no `nsfw` score is present.
    pub fn from_model_scores(
        resource: ResourceRef,
        raw_scores: Vec<SafetyModelScore>,
    ) -> SafetyResult<Self> {
        let score = raw_scores
            .iter()
            .find(|entry| entry.label == "nsfw")
            .map(|entry| entry.score)
            .ok_or_else(|| SafetyError::scanner("model scores must include an nsfw score"))?;
        let safe_score = raw_scores
            .iter()
            .find(|entry| entry.label == "safe")
            .map(|entry| entry.score);
        Ok(Self {
            resource,
            score,
            safe_score,
            raw_scores,
            scorer_label: None,
            scorer_version: None,
            assessed_at_ms: None,
        })
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

    #[must_use]
    pub fn risk_band(&self) -> SafetyRiskBand {
        let value = self.score.value();
        if value < 0.20 {
            SafetyRiskBand::Low
        } else if value < 0.80 {
            SafetyRiskBand::Medium
        } else {
            SafetyRiskBand::High
        }
    }

    #[must_use]
    pub fn auto_label(&self) -> SafetyLabel {
        match self.risk_band() {
            SafetyRiskBand::Low | SafetyRiskBand::Medium => SafetyLabel::Safe,
            SafetyRiskBand::High => SafetyLabel::Sensitive,
        }
    }

    #[must_use]
    pub fn effective_label(&self, manual_override: Option<SafetyLabel>) -> SafetyLabel {
        manual_override.unwrap_or_else(|| self.auto_label())
    }
}
