use atelier_adapter_novelai::NovelAiClientFactory;
use atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, GetVibeDocumentRequestDto, ImportEmbeddedPngVibeDocumentRequestDto,
    ImportVibeDocumentRequestDto, ImportedVibeDocumentsDto, ListVibeDocumentsRequestDto,
    RenameVibeDocumentRequestDto, SetVibeDocumentHiddenRequestDto, VibeDocumentEntryDto,
    VibeDocumentPageDto,
};
use atelier_secrets::SecretStore;
use atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AtelierRuntime, CommandResult};

impl<S, F, E> AtelierRuntime<S, F, E>
where
    S: SecretStore + Clone + Send + Sync,
    F: NovelAiClientFactory + Clone + Send + Sync,
    E: EmbeddedVibeDocumentExtractor + Clone + Send + Sync,
{
    /// Imports official `NovelAI` Vibe JSON.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the document is invalid, or resource storage fails.
    pub async fn import_vibe_document(
        &self,
        request: ImportVibeDocumentRequestDto,
    ) -> CommandResult<ImportedVibeDocumentsDto> {
        Self::command_result(
            self.current_session()?
                .vibe()
                .import_document(request)
                .await,
        )
    }

    /// Imports a Vibe document embedded in PNG metadata.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, PNG extraction fails, or resource storage fails.
    pub async fn import_embedded_png_vibe_document(
        &self,
        request: ImportEmbeddedPngVibeDocumentRequestDto,
    ) -> CommandResult<ImportedVibeDocumentsDto> {
        Self::command_result(
            self.current_session()?
                .vibe()
                .import_embedded_png(request)
                .await,
        )
    }

    /// Exports managed Vibe documents as official `NovelAI` JSON.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, a Vibe is missing, or export fails.
    pub async fn export_vibe_document(
        &self,
        request: ExportVibeDocumentRequestDto,
    ) -> CommandResult<ExportedVibeDocumentDto> {
        Self::command_result(
            self.current_session()?
                .vibe()
                .export_document(request)
                .await,
        )
    }

    /// Lists workspace-scoped Vibe documents available to generation.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the Vibe catalog cannot be read.
    pub async fn list_vibe_documents(
        &self,
        request: ListVibeDocumentsRequestDto,
    ) -> CommandResult<VibeDocumentPageDto> {
        Self::command_result(self.current_session()?.vibe().list_documents(request).await)
    }

    /// Reads one workspace-scoped Vibe document entry.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the Vibe document does not exist.
    pub async fn get_vibe_document(
        &self,
        request: GetVibeDocumentRequestDto,
    ) -> CommandResult<VibeDocumentEntryDto> {
        Self::command_result(self.current_session()?.vibe().get_document(request).await)
    }

    /// Ensures a Vibe encoding exists for the requested model/settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, no active API key exists, or encoding fails.
    pub async fn ensure_vibe_encoding(
        &self,
        request: EnsureVibeEncodingRequestDto,
    ) -> CommandResult<EnsuredVibeEncodingDto> {
        Self::command_result(
            self.current_session()?
                .vibe()
                .ensure_encoding(request)
                .await,
        )
    }

    /// Renames a managed Vibe document.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, the name is invalid, or the Vibe document is missing.
    pub async fn rename_vibe_document(
        &self,
        request: RenameVibeDocumentRequestDto,
    ) -> CommandResult<VibeDocumentEntryDto> {
        Self::command_result(
            self.current_session()?
                .vibe()
                .rename_document(request)
                .await,
        )
    }

    /// Soft-hides or restores a managed Vibe document.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open or the Vibe document is missing.
    pub async fn set_vibe_document_hidden(
        &self,
        request: SetVibeDocumentHiddenRequestDto,
    ) -> CommandResult<VibeDocumentEntryDto> {
        Self::command_result(
            self.current_session()?
                .vibe()
                .set_document_hidden(request)
                .await,
        )
    }
}
