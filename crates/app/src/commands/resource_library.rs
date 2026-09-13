use atelier_app_api::resource_library::{
    DeleteLibraryFolderRequestDto, DeleteLibraryFolderResponseDto, GetLibrarySnapshotRequestDto,
    LibraryFolderDto, LibraryResourceDto, LibrarySnapshotDto, UpdateLibraryResourceRequestDto,
    UpsertLibraryFolderRequestDto,
};

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: Send + Sync,
    F: Send + Sync,
    E: Send + Sync,
{
    /// Returns the path-resolved tree for one resource namespace.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or storage fails.
    pub async fn get_resource_library_snapshot(
        &self,
        request: GetLibrarySnapshotRequestDto,
    ) -> CommandResult<LibrarySnapshotDto> {
        Self::command_result(
            self.current_session()?
                .resource_library()
                .snapshot(request)
                .await,
        )
    }

    /// Creates, renames, or moves one logical folder.
    ///
    /// # Errors
    /// Returns an error envelope for invalid ancestry, conflicts, or storage failures.
    pub async fn upsert_resource_library_folder(
        &self,
        request: UpsertLibraryFolderRequestDto,
    ) -> CommandResult<LibraryFolderDto> {
        Self::command_result(
            self.current_session()?
                .resource_library()
                .upsert_folder(request)
                .await,
        )
    }

    /// Renames or moves one logical resource.
    ///
    /// # Errors
    /// Returns an error envelope for invalid metadata, conflicts, or storage failures.
    pub async fn update_resource_library_resource(
        &self,
        request: UpdateLibraryResourceRequestDto,
    ) -> CommandResult<LibraryResourceDto> {
        Self::command_result(
            self.current_session()?
                .resource_library()
                .update_resource(request)
                .await,
        )
    }

    /// Deletes one folder subtree after reference-safety checks.
    ///
    /// # Errors
    /// Returns an error envelope when the subtree is referenced or cannot be deleted.
    pub async fn delete_resource_library_folder(
        &self,
        request: DeleteLibraryFolderRequestDto,
    ) -> CommandResult<DeleteLibraryFolderResponseDto> {
        Self::command_result(
            self.current_session()?
                .resource_library()
                .delete_folder(request)
                .await,
        )
    }
}
