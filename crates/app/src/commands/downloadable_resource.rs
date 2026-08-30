use atelier_app_api::downloadable_resource::{
    DownloadableResourceGroupDto, DownloadableResourceGroupRequestDto,
    DownloadableResourceRequestDto, DownloadableResourceStateDto, DownloadableResourceStatusDto,
    DownloadableResourcesDto,
};
use atelier_downloadable_resources::{
    DownloadableResourceCatalog, DownloadableResourceManager, DownloadableResourceState,
    DownloadableResourceStatus, ResourceInstallProgressSink,
};
use atelier_image_analysis::ImageAnalysisModelId;

use crate::commands::{AtelierRuntime, CommandResult};
use crate::{AppError, AppResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Lists catalog groups and current resource states.
    ///
    /// # Errors
    /// Returns an error envelope when the catalog or manager state is unavailable.
    pub async fn list_downloadable_resources(&self) -> CommandResult<DownloadableResourcesDto> {
        self.resource_snapshot(false)
            .await
            .map_err(|error| error.envelope())
    }

    /// Refreshes the remote catalog and returns a new snapshot.
    ///
    /// # Errors
    /// Returns an error envelope when no remote or cached catalog is available.
    pub async fn refresh_downloadable_resource_catalog(
        &self,
    ) -> CommandResult<DownloadableResourcesDto> {
        self.resource_snapshot(true)
            .await
            .map_err(|error| error.envelope())
    }

    /// Marks the first-run resource selection as completed or skipped.
    ///
    /// # Errors
    /// Returns an error envelope when the marker cannot be persisted.
    pub fn complete_downloadable_resource_onboarding(&self) -> CommandResult<()> {
        self.resources()?
            .complete_onboarding()
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Installs one resource and its dependencies.
    ///
    /// # Errors
    /// Returns an error envelope when download, verification, or activation fails.
    pub async fn install_downloadable_resource(
        &self,
        request: DownloadableResourceRequestDto,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> CommandResult<DownloadableResourceStatusDto> {
        self.resources()?
            .install(&request.resource_id, progress)
            .await
            .map(status_to_dto)
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Installs every resource in an ordered catalog group.
    ///
    /// # Errors
    /// Returns an error envelope when any resource cannot be installed.
    pub async fn install_downloadable_resource_group(
        &self,
        request: DownloadableResourceGroupRequestDto,
        progress: Option<&dyn ResourceInstallProgressSink>,
    ) -> CommandResult<Vec<DownloadableResourceStatusDto>> {
        self.resources()?
            .install_group(&request.group_id, progress)
            .await
            .map(|values| values.into_iter().map(status_to_dto).collect())
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Cancels an active installation.
    ///
    /// # Errors
    /// Returns an error envelope when manager state is unavailable.
    #[allow(clippy::needless_pass_by_value, reason = "Tauri DTO command boundary")]
    pub fn cancel_downloadable_resource_install(
        &self,
        request: DownloadableResourceRequestDto,
    ) -> CommandResult<()> {
        self.resources()?
            .cancel_install(&request.resource_id)
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    /// Removes an active resource, deferring file deletion while leases exist.
    ///
    /// # Errors
    /// Returns an error envelope when the resource is active in settings or deletion fails.
    pub async fn delete_downloadable_resource(
        &self,
        request: DownloadableResourceRequestDto,
    ) -> CommandResult<()> {
        if request.resource_id == "wd-swinv2-tagger-v3"
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
                "downloadable_resource_in_use",
                "disable WD automatic review before deleting its resource",
            )
            .envelope());
        }
        if let Some(sessions) = &self.image_analysis_sessions
            && let Some(model) = model_for_resource(&request.resource_id)
        {
            sessions
                .unload(model)
                .map_err(AppError::from)
                .map_err(|error| error.envelope())?;
        }
        self.resources()?
            .delete(&request.resource_id)
            .await
            .map_err(AppError::from)
            .map_err(|error| error.envelope())
    }

    async fn resource_snapshot(&self, refresh: bool) -> AppResult<DownloadableResourcesDto> {
        let resources = self
            .resources()
            .map_err(|error| AppError::new(error.code, error.message))?;
        let catalog = resources.catalog(refresh).await?;
        let statuses = resources.statuses().await?;
        Ok(snapshot_to_dto(
            &catalog,
            statuses,
            resources.onboarding_complete()?,
        ))
    }

    fn resources(&self) -> CommandResult<&dyn DownloadableResourceManager> {
        self.downloadable_resources.as_deref().ok_or_else(|| {
            AppError::new(
                "downloadable_resources_unavailable",
                "downloadable resource management is unavailable",
            )
            .envelope()
        })
    }
}

fn snapshot_to_dto(
    catalog: &DownloadableResourceCatalog,
    statuses: Vec<DownloadableResourceStatus>,
    onboarding_complete: bool,
) -> DownloadableResourcesDto {
    let groups = catalog
        .groups
        .iter()
        .map(|group| DownloadableResourceGroupDto {
            id: group.id.clone(),
            resources: group.resources.clone(),
            size_bytes: group
                .resources
                .iter()
                .filter_map(|id| catalog.resources.iter().find(|resource| &resource.id == id))
                .map(atelier_downloadable_resources::DownloadableResourceDescriptor::size_bytes)
                .sum(),
        })
        .collect();
    DownloadableResourcesDto {
        catalog_version: catalog.catalog_version.clone(),
        onboarding_complete,
        resources: statuses.into_iter().map(status_to_dto).collect(),
        groups,
    }
}

fn status_to_dto(value: DownloadableResourceStatus) -> DownloadableResourceStatusDto {
    DownloadableResourceStatusDto {
        id: value.id,
        available_version: value.available_version,
        installed_version: value.installed_version,
        state: match value.state {
            DownloadableResourceState::Missing => DownloadableResourceStateDto::Missing,
            DownloadableResourceState::Downloading => DownloadableResourceStateDto::Downloading,
            DownloadableResourceState::Verifying => DownloadableResourceStateDto::Verifying,
            DownloadableResourceState::Ready => DownloadableResourceStateDto::Ready,
            DownloadableResourceState::UpdateAvailable => {
                DownloadableResourceStateDto::UpdateAvailable
            }
            DownloadableResourceState::Failed => DownloadableResourceStateDto::Failed,
        },
        size_bytes: value.size_bytes,
        downloaded_bytes: value.downloaded_bytes,
        message: value.message,
    }
}

fn model_for_resource(id: &str) -> Option<ImageAnalysisModelId> {
    match id {
        "anime-dbrating" => Some(ImageAnalysisModelId::AnimeDbRating),
        "wd-swinv2-tagger-v3" => Some(ImageAnalysisModelId::WdSwinv2TaggerV3),
        _ => None,
    }
}
