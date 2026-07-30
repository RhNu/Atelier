use atelier_app_api::image_analysis::{ImageAnalysisModelRequestDto, ImageAnalysisModelStatusDto};
use atelier_image_analysis::{ImageAnalysisModelManager, ModelInstallProgressSink};

use crate::AppError;
use crate::commands::{AtelierRuntime, CommandResult};
use crate::mapping::{image_analysis_model_id_to_domain, image_analysis_model_status_to_dto};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Returns installation and validation status for all known image-analysis models.
    ///
    /// # Errors
    /// Returns an error when image analysis is unavailable or model state cannot be read.
    pub async fn get_image_analysis_model_status(
        &self,
    ) -> CommandResult<Vec<ImageAnalysisModelStatusDto>> {
        self.models()?
            .statuses()
            .await
            .map(|statuses| {
                statuses
                    .into_iter()
                    .map(image_analysis_model_status_to_dto)
                    .collect()
            })
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Installs one pinned image-analysis model.
    ///
    /// # Errors
    /// Returns an error when the model is unknown, download fails, or verification fails.
    pub async fn install_image_analysis_model(
        &self,
        request: ImageAnalysisModelRequestDto,
        progress: Option<&dyn ModelInstallProgressSink>,
    ) -> CommandResult<ImageAnalysisModelStatusDto> {
        self.models()?
            .install(
                image_analysis_model_id_to_domain(request.model_id),
                progress,
            )
            .await
            .map(image_analysis_model_status_to_dto)
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Cancels an active model installation.
    ///
    /// # Errors
    /// Returns an error when image analysis is unavailable.
    pub fn cancel_image_analysis_model_install(
        &self,
        request: ImageAnalysisModelRequestDto,
    ) -> CommandResult<()> {
        self.models()?
            .cancel_install(image_analysis_model_id_to_domain(request.model_id))
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Deletes an optional model package after unloading its session.
    ///
    /// # Errors
    /// Returns an error for required models or when deletion fails.
    pub async fn delete_image_analysis_model(
        &self,
        request: ImageAnalysisModelRequestDto,
    ) -> CommandResult<()> {
        let model = image_analysis_model_id_to_domain(request.model_id);
        if model == atelier_image_analysis::ImageAnalysisModelId::WdSwinv2TaggerV3
            && self
                .global_settings
                .get_global_settings()
                .await
                .map_err(AppError::from)
                .map_err(|error| error.envelope())?
                .safety
                .wd_auto_review_enabled
        {
            return Err(AppError::new(
                "image_analysis_model_in_use",
                "disable WD automatic review before deleting the WD model",
            )
            .envelope());
        }
        self.models()?
            .delete(model)
            .await
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    fn models(&self) -> CommandResult<&dyn ImageAnalysisModelManager> {
        self.image_analysis_models.as_deref().ok_or_else(|| {
            AppError::new(
                "image_analysis_unavailable",
                "image analysis model management is unavailable",
            )
            .envelope()
        })
    }
}
