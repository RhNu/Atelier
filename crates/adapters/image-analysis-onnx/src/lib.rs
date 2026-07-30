//! Downloaded model-pack management and ONNX image analysis.

mod analyzer;
mod manifest;
mod model_store;
mod preprocess;
mod spec;

pub use atelier_adapter_onnx_runtime::OrtRuntime;
pub use model_store::OnnxImageAnalysisRuntime;
pub use spec::{
    ANIME_DBRATING_REVISION, WD_TAGGER_REVISION, model_spec, runtime_library_file_name,
};

use std::path::Path;

use atelier_image_analysis::{ImageAnalysisError, ImageAnalysisResult};

/// Initializes the shared process-wide ONNX Runtime library.
///
/// # Errors
/// Returns an error when the host-selected library cannot be initialized.
pub fn initialize_ort_runtime(
    runtime_library_path: impl AsRef<Path>,
) -> ImageAnalysisResult<&'static OrtRuntime> {
    atelier_adapter_onnx_runtime::initialize(runtime_library_path)
        .map_err(|error| ImageAnalysisError::inference(error.to_string()))
}
