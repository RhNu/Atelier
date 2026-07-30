use atelier_image_analysis::{
    AnalysisOutputSelection, ImageConfidence, ImageRatingScores, ImageTagCategory, ImageTagScore,
};

#[test]
fn confidence_rejects_non_finite_and_out_of_range_values() {
    assert!(ImageConfidence::new(f32::NAN).is_err());
    assert!(ImageConfidence::new(-0.01).is_err());
    assert!(ImageConfidence::new(1.01).is_err());
    assert!((ImageConfidence::new(0.5).unwrap().value() - 0.5).abs() < f32::EPSILON);
}

#[test]
fn rating_scores_preserve_the_typed_output_order() {
    let scores = ImageRatingScores::new(0.1, 0.2, 0.3, 0.4).unwrap();

    for (actual, expected) in scores.values().into_iter().zip([0.1, 0.2, 0.3, 0.4]) {
        assert!((actual - expected).abs() < f32::EPSILON);
    }
}

#[test]
fn tag_score_preserves_source_identity_and_category() {
    let tag = ImageTagScore::new(470_575, "1girl", ImageTagCategory::General, 0.98).unwrap();

    assert_eq!(tag.tag_id, 470_575);
    assert_eq!(tag.canonical_name, "1girl");
    assert_eq!(tag.category, ImageTagCategory::General);
}

#[test]
fn output_selection_can_request_ratings_without_materializing_tags() {
    let outputs = AnalysisOutputSelection::ratings_only();

    assert!(outputs.ratings);
    assert!(!outputs.general_tags);
    assert!(!outputs.character_tags);
    assert!(!outputs.is_empty());
}
