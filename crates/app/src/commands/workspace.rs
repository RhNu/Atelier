use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::workspace::{
    AppBootstrapDto, CloseWorkspaceResponseDto, OpenWorkspaceRequestDto,
    WorkspaceRestoreFailureDto, WorkspaceStatusDto,
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AtelierRuntime, CommandResult};
use crate::mapping::global_settings_to_dto;

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync + 'static,
    F: NovelAiClientFactory + Clone + Send + Sync + 'static,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync + 'static,
{
    /// Opens a workspace and replaces any existing command session.
    ///
    /// # Errors
    /// Returns an error envelope when workspace initialization, locking, database setup, or lexicon loading fails.
    pub async fn open_workspace(
        &self,
        request: OpenWorkspaceRequestDto,
    ) -> CommandResult<WorkspaceStatusDto> {
        if let Some(session) = self.current_session_optional()?
            && session.inner.root.as_path() == request.root.as_path()
        {
            self.global_settings
                .record_last_workspace(request.root)
                .await
                .map_err(crate::AppError::from)
                .map_err(|error| error.envelope())?;
            return Ok(session.workspace().status());
        }

        let session = self.build_session(request.root.clone()).await?;
        self.global_settings
            .record_last_workspace(request.root)
            .await
            .map_err(crate::AppError::from)
            .map_err(|error| error.envelope())?;
        let status = session.workspace().status();
        self.publish_session(session)?;
        Ok(status)
    }

    /// Loads global settings and restores the most recently opened workspace when possible.
    ///
    /// # Errors
    /// Returns an error envelope when global settings cannot be read or runtime state is unavailable.
    pub async fn bootstrap_app(&self) -> CommandResult<AppBootstrapDto> {
        let settings = self
            .global_settings
            .get_global_settings()
            .await
            .map_err(crate::AppError::from)
            .map_err(|error| error.envelope())?;
        let Some(root) = settings.last_workspace.clone() else {
            return Ok(AppBootstrapDto {
                global_settings: global_settings_to_dto(&settings),
                workspace: None,
                restore_failure: None,
            });
        };

        if let Some(session) = self.current_session_optional()?
            && session.inner.root.as_path() == root.as_path()
        {
            return Ok(AppBootstrapDto {
                global_settings: global_settings_to_dto(&settings),
                workspace: Some(session.workspace().status()),
                restore_failure: None,
            });
        }

        match self.build_session(root.clone()).await {
            Ok(session) => {
                let status = session.workspace().status();
                self.publish_session(session)?;
                Ok(AppBootstrapDto {
                    global_settings: global_settings_to_dto(&settings),
                    workspace: Some(status),
                    restore_failure: None,
                })
            }
            Err(error) => Ok(AppBootstrapDto {
                global_settings: global_settings_to_dto(&settings),
                workspace: None,
                restore_failure: Some(WorkspaceRestoreFailureDto { root, error }),
            }),
        }
    }
}

impl<S, F, E> AtelierRuntime<S, F, E> {
    /// Returns the current workspace status.
    ///
    /// # Errors
    /// Returns an error envelope when runtime state is unavailable.
    pub fn workspace_status(&self) -> CommandResult<Option<WorkspaceStatusDto>> {
        Ok(self
            .current_session_optional()?
            .map(|session| session.workspace().status()))
    }

    /// Closes the current workspace session, if any.
    ///
    /// # Errors
    /// Returns an error envelope when command session state is unavailable.
    pub fn close_workspace(&self) -> CommandResult<CloseWorkspaceResponseDto> {
        let session = self.lock_session()?.take();
        let Some(session) = session else {
            return Ok(CloseWorkspaceResponseDto { was_open: false });
        };
        if let Err(error) = session.release_workspace_lock() {
            // Put the session back so callers can retry a failed close.
            *self.lock_session()? = Some(session);
            return Err(error.envelope());
        }
        Ok(CloseWorkspaceResponseDto { was_open: true })
    }
}
