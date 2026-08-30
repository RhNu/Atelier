use atelier_resource_catalog::ResourceRef;

use crate::{ImageAnalysisError, ImageAnalysisResult};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ImageConfidence(f32);

impl ImageConfidence {
    /// Creates a finite confidence in the inclusive range `0.0..=1.0`.
    ///
    /// # Errors
    /// Returns an error when `value` is non-finite or outside the supported range.
    pub fn new(value: f32) -> ImageAnalysisResult<Self> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(ImageAnalysisError::invalid_score(
                "image confidence must be between 0.0 and 1.0",
            ))
        }
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ImageAnalysisModelId {
    AnimeDbRating,
    WdSwinv2TaggerV3,
}

impl ImageAnalysisModelId {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AnimeDbRating => "anime_dbrating",
            Self::WdSwinv2TaggerV3 => "wd_swinv2_tagger_v3",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnalysisOutputSelection {
    pub ratings: bool,
    pub general_tags: bool,
    pub character_tags: bool,
}

impl AnalysisOutputSelection {
    #[must_use]
    pub const fn ratings_only() -> Self {
        Self {
            ratings: true,
            general_tags: false,
            character_tags: false,
        }
    }

    #[must_use]
    pub const fn all() -> Self {
        Self {
            ratings: true,
            general_tags: true,
            character_tags: true,
        }
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.ratings && !self.general_tags && !self.character_tags
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageAnalysisInput {
    pub resource: ResourceRef,
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ImageRatingScores {
    pub general: ImageConfidence,
    pub sensitive: ImageConfidence,
    pub questionable: ImageConfidence,
    pub explicit: ImageConfidence,
}

impl ImageRatingScores {
    /// Creates the four Danbooru-style rating scores.
    ///
    /// # Errors
    /// Returns an error when any score is non-finite or outside `0.0..=1.0`.
    pub fn new(
        general: f32,
        sensitive: f32,
        questionable: f32,
        explicit: f32,
    ) -> ImageAnalysisResult<Self> {
        Ok(Self {
            general: ImageConfidence::new(general)?,
            sensitive: ImageConfidence::new(sensitive)?,
            questionable: ImageConfidence::new(questionable)?,
            explicit: ImageConfidence::new(explicit)?,
        })
    }

    #[must_use]
    pub const fn values(self) -> [f32; 4] {
        [
            self.general.value(),
            self.sensitive.value(),
            self.questionable.value(),
            self.explicit.value(),
        ]
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageTagCategory {
    General,
    Character,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageTagScore {
    pub tag_id: u64,
    pub canonical_name: String,
    pub category: ImageTagCategory,
    pub confidence: ImageConfidence,
}

impl ImageTagScore {
    /// Creates one model tag while preserving the upstream Danbooru tag ID.
    ///
    /// # Errors
    /// Returns an error for an empty name or invalid confidence.
    pub fn new(
        tag_id: u64,
        canonical_name: impl Into<String>,
        category: ImageTagCategory,
        confidence: f32,
    ) -> ImageAnalysisResult<Self> {
        let canonical_name = canonical_name.into();
        if canonical_name.trim().is_empty() {
            return Err(ImageAnalysisError::invalid_request(
                "image tag name must not be empty",
            ));
        }
        Ok(Self {
            tag_id,
            canonical_name,
            category,
            confidence: ImageConfidence::new(confidence)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageAnalysisModelInfo {
    pub id: ImageAnalysisModelId,
    pub revision: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageAnalysis {
    pub resource: ResourceRef,
    pub model: ImageAnalysisModelInfo,
    pub ratings: Option<ImageRatingScores>,
    pub general_tags: Vec<ImageTagScore>,
    pub character_tags: Vec<ImageTagScore>,
}
