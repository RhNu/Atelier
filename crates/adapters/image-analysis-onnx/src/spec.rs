pub const ANIME_DBRATING_RESOURCE_ID: &str = "anime-dbrating";
pub const WD_TAGGER_RESOURCE_ID: &str = "wd-swinv2-tagger-v3";
pub const ANIME_DBRATING_REVISION: &str = "7af21db648acdeb74f5c334abda9dd7403407b3c";
pub const WD_TAGGER_REVISION: &str = "627aef95638667ddcaa3ac8ae625e88ea5b02f51";

#[cfg(target_os = "windows")]
#[must_use]
pub const fn runtime_library_file_name() -> &'static str {
    "onnxruntime.dll"
}

#[cfg(target_os = "linux")]
#[must_use]
pub const fn runtime_library_file_name() -> &'static str {
    "libonnxruntime.so"
}

#[cfg(target_os = "macos")]
#[must_use]
pub const fn runtime_library_file_name() -> &'static str {
    "libonnxruntime.dylib"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_model_revisions_match_the_versioned_safety_policy() {
        let policy = atelier_safety::anime_rating_policy();
        assert_eq!(ANIME_DBRATING_REVISION, policy.primary_model_revision);
        assert_eq!(WD_TAGGER_REVISION, policy.review_model_revision);
    }
}
