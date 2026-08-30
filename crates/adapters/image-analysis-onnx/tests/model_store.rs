use atelier_adapter_image_analysis_onnx::{
    ANIME_DBRATING_RESOURCE_ID, ANIME_DBRATING_REVISION, WD_TAGGER_RESOURCE_ID, WD_TAGGER_REVISION,
};

#[test]
fn model_resources_preserve_pinned_upstream_evidence() {
    assert_eq!(ANIME_DBRATING_RESOURCE_ID, "anime-dbrating");
    assert_eq!(WD_TAGGER_RESOURCE_ID, "wd-swinv2-tagger-v3");
    assert_eq!(ANIME_DBRATING_REVISION.len(), 40);
    assert_eq!(WD_TAGGER_REVISION.len(), 40);
}
