use async_trait::async_trait;

use crate::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisInput, ImageAnalysisModelId,
    ImageAnalysisResult,
};

#[async_trait]
pub trait ImageAnalyzer: Send + Sync {
    /// Runs the selected model and returns only the requested output families.
    ///
    /// # Errors
    ///
    /// Returns an error when the model is unavailable, input decoding fails, or inference fails.
    async fn analyze(
        &self,
        model: ImageAnalysisModelId,
        input: ImageAnalysisInput,
        outputs: AnalysisOutputSelection,
    ) -> ImageAnalysisResult<ImageAnalysis>;
}

#[async_trait]
pub trait ImageAnalysisSessionControl: Send + Sync {
    /// Releases a loaded inference session after any in-flight inference completes.
    ///
    /// # Errors
    ///
    /// Returns an error when the session cannot be released.
    fn unload(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()>;
}
