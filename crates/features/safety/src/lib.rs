//! Versioned safety policy and scanning contracts for image resources.

mod error;
mod model;
mod pipeline;
mod policy;
mod ports;

pub use error::{SafetyError, SafetyErrorKind, SafetyResult};
pub use model::{
    ImageSafetyScore, SafetyAssessment, SafetyLabel, SafetyModelEvidence, SafetyReviewOutcome,
    SafetyRiskBand,
};
pub use pipeline::{SafetyPipeline, SafetyPolicyControl};
pub use policy::{
    ANIME_RATING_POLICY_ID, ANIME_RATING_POLICY_VERSION, AnimeRatingPolicy, anime_rating_policy,
};
pub use ports::{SafetyScanInput, SafetyScanner};
