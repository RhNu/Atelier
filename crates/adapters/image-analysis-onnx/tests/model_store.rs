use atelier_adapter_image_analysis_onnx::{ANIME_DBRATING_REVISION, model_spec};
use atelier_image_analysis::ImageAnalysisModelId;

#[test]
fn pinned_specs_preserve_revision_and_required_roles() {
    let primary = model_spec(ImageAnalysisModelId::AnimeDbRating);
    let review = model_spec(ImageAnalysisModelId::WdSwinv2TaggerV3);

    assert_eq!(primary.revision, ANIME_DBRATING_REVISION);
    assert!(primary.required);
    assert!(!review.required);
    assert_eq!(primary.files.len(), 2);
    assert_eq!(review.files.len(), 2);
}
