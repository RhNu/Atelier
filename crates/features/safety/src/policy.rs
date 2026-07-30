use std::sync::OnceLock;

use atelier_image_analysis::ImageRatingScores;
use serde::Deserialize;

pub const ANIME_RATING_POLICY_ID: &str = "anime-rating-cascade";
pub const ANIME_RATING_POLICY_VERSION: &str = "1";

const POLICY_JSON: &str = include_str!("../policies/anime-rating-cascade-v1.json");

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AnimeRatingPolicy {
    pub id: String,
    pub version: String,
    pub primary_model_revision: String,
    pub review_model_revision: String,
    primary_intercept: f32,
    primary_coefficients: [f32; 4],
    pub primary_threshold: f32,
    pub review_margin: f32,
    review_intercept: f32,
    review_coefficients: [f32; 4],
    pub review_threshold: f32,
}

impl AnimeRatingPolicy {
    #[must_use]
    pub fn primary_score(&self, ratings: ImageRatingScores) -> f32 {
        logistic_score(ratings, self.primary_intercept, self.primary_coefficients)
    }

    #[must_use]
    pub fn review_score(&self, ratings: ImageRatingScores) -> f32 {
        logistic_score(ratings, self.review_intercept, self.review_coefficients)
    }

    #[must_use]
    pub fn should_review(&self, primary_score: f32) -> bool {
        (self.primary_threshold - self.review_margin..self.primary_threshold)
            .contains(&primary_score)
    }
}

#[must_use]
/// Returns the immutable production policy embedded in this build.
///
/// # Panics
/// Panics only when the checked-in policy asset is malformed or its identity does not match the
/// exported policy constants.
pub fn anime_rating_policy() -> &'static AnimeRatingPolicy {
    static POLICY: OnceLock<AnimeRatingPolicy> = OnceLock::new();
    POLICY.get_or_init(|| {
        let policy: AnimeRatingPolicy =
            serde_json::from_str(POLICY_JSON).expect("embedded anime rating policy must be valid");
        assert_eq!(policy.id, ANIME_RATING_POLICY_ID);
        assert_eq!(policy.version, ANIME_RATING_POLICY_VERSION);
        policy
    })
}

fn logistic_score(ratings: ImageRatingScores, intercept: f32, coefficients: [f32; 4]) -> f32 {
    let logit = ratings
        .values()
        .into_iter()
        .zip(coefficients)
        .fold(intercept, |sum, (score, coefficient)| {
            score.mul_add(coefficient, sum)
        });
    1.0 / (1.0 + (-logit).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_policy_has_the_exported_identity_and_a_valid_review_band() {
        let policy = anime_rating_policy();

        assert_eq!(policy.id, ANIME_RATING_POLICY_ID);
        assert_eq!(policy.version, ANIME_RATING_POLICY_VERSION);
        assert!(policy.review_margin > 0.0);
        assert!(policy.review_margin < policy.primary_threshold);
    }
}
