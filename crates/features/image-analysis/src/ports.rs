use async_trait::async_trait;

use crate::{
    AnalysisOutputSelection, ImageAnalysis, ImageAnalysisInput, ImageAnalysisModelId,
    ImageAnalysisModelStatus, ImageAnalysisResult, ModelInstallProgress,
};

pub trait ModelInstallProgressSink: Send + Sync {
    /// Receives the latest installation progress snapshot.
    fn report(&self, progress: ModelInstallProgress);
}

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
pub trait ImageAnalysisModelManager: Send + Sync {
    /// Returns the current state of every supported model.
    ///
    /// # Errors
    ///
    /// Returns an error when the model store cannot be inspected.
    async fn statuses(&self) -> ImageAnalysisResult<Vec<ImageAnalysisModelStatus>>;

    /// Installs and verifies a model package.
    ///
    /// # Errors
    ///
    /// Returns an error when downloading, verification, or activation fails.
    async fn install(
        &self,
        model: ImageAnalysisModelId,
        progress: Option<&dyn ModelInstallProgressSink>,
    ) -> ImageAnalysisResult<ImageAnalysisModelStatus>;

    /// Requests cancellation of an in-progress model installation.
    ///
    /// # Errors
    ///
    /// Returns an error when the cancellation state cannot be updated.
    fn cancel_install(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()>;

    /// Unloads and deletes an optional model package.
    ///
    /// # Errors
    ///
    /// Returns an error when the model is required, still enabled, or cannot be deleted.
    async fn delete(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()>;

    /// Releases a loaded inference session after any in-flight inference completes.
    ///
    /// # Errors
    ///
    /// Returns an error when the session cannot be released.
    fn unload(&self, model: ImageAnalysisModelId) -> ImageAnalysisResult<()>;

    /// Returns whether a model package is installed and verified.
    async fn is_ready(&self, model: ImageAnalysisModelId) -> bool;
}
