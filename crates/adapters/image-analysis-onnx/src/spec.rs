use atelier_image_analysis::ImageAnalysisModelId;

pub const ANIME_DBRATING_REVISION: &str = "7af21db648acdeb74f5c334abda9dd7403407b3c";
pub const WD_TAGGER_REVISION: &str = "627aef95638667ddcaa3ac8ae625e88ea5b02f51";

#[derive(Copy, Clone, Debug)]
pub struct ModelFileSpec {
    pub relative_path: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
    pub size_bytes: u64,
}

#[derive(Copy, Clone, Debug)]
pub struct ModelSpec {
    pub id: ImageAnalysisModelId,
    pub revision: &'static str,
    pub required: bool,
    pub files: &'static [ModelFileSpec],
}

const DBRATING_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        relative_path: "model.onnx",
        url: "https://huggingface.co/deepghs/anime_dbrating/resolve/7af21db648acdeb74f5c334abda9dd7403407b3c/mobilenetv3_large_100_v0_ls0.2/model.onnx?download=true",
        sha256: "c7d6fd0cd71c48616fa87fe2bca87f8e1775e9a3b96d12797e2144fca3543362",
        size_bytes: 16_832_684,
    },
    ModelFileSpec {
        relative_path: "meta.json",
        url: "https://huggingface.co/deepghs/anime_dbrating/resolve/7af21db648acdeb74f5c334abda9dd7403407b3c/mobilenetv3_large_100_v0_ls0.2/meta.json?download=true",
        sha256: "65d5d69ef309eba3d04e5058f9bafcd7b33a707a05adaa93f09d8c1655faf163",
        size_bytes: 169,
    },
];

const WD_FILES: &[ModelFileSpec] = &[
    ModelFileSpec {
        relative_path: "model.onnx",
        url: "https://huggingface.co/SmilingWolf/wd-swinv2-tagger-v3/resolve/627aef95638667ddcaa3ac8ae625e88ea5b02f51/model.onnx?download=true",
        sha256: "e6774bff34d43bd49f75a47db4ef217dce701c9847b546523eb85ff6dbba1db1",
        size_bytes: 467_460_978,
    },
    ModelFileSpec {
        relative_path: "selected_tags.csv",
        url: "https://huggingface.co/SmilingWolf/wd-swinv2-tagger-v3/resolve/627aef95638667ddcaa3ac8ae625e88ea5b02f51/selected_tags.csv?download=true",
        sha256: "298633d94d0031d2081c0893f29c82eab7f0df00b08483ba8f29d1e979441217",
        size_bytes: 308_468,
    },
];

const DBRATING_SPEC: ModelSpec = ModelSpec {
    id: ImageAnalysisModelId::AnimeDbRating,
    revision: ANIME_DBRATING_REVISION,
    required: true,
    files: DBRATING_FILES,
};

const WD_SPEC: ModelSpec = ModelSpec {
    id: ImageAnalysisModelId::WdSwinv2TaggerV3,
    revision: WD_TAGGER_REVISION,
    required: false,
    files: WD_FILES,
};

#[must_use]
pub const fn model_spec(id: ImageAnalysisModelId) -> &'static ModelSpec {
    match id {
        ImageAnalysisModelId::AnimeDbRating => &DBRATING_SPEC,
        ImageAnalysisModelId::WdSwinv2TaggerV3 => &WD_SPEC,
    }
}

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
