use nai_atelier_adapter_novelai::NovelAiClientFactory;
use nai_atelier_app_api::vibe::{
    EnsureVibeEncodingRequestDto, EnsuredVibeEncodingDto, ExportVibeDocumentRequestDto,
    ExportedVibeDocumentDto, ImportEmbeddedPngVibeDocumentRequestDto, ImportVibeDocumentRequestDto,
    ImportedVibeDocumentsDto,
};
use nai_atelier_secrets::SecretStore;
use nai_atelier_vibe::EmbeddedVibeDocumentExtractor;

use crate::commands::{AppCommandHost, CommandResult};

impl<S, F, E> AppCommandHost<S, F, E>
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
        Self::command_result(self.current_app()?.vibe().import_document(request).await)
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
            self.current_app()?
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
        Self::command_result(self.current_app()?.vibe().export_document(request).await)
    }

    /// Ensures a Vibe encoding exists for the requested model/settings.
    ///
    /// # Errors
    /// Returns an error envelope when no workspace is open, no active API key exists, or encoding fails.
    pub async fn ensure_vibe_encoding(
        &self,
        request: EnsureVibeEncodingRequestDto,
    ) -> CommandResult<EnsuredVibeEncodingDto> {
        Self::command_result(self.current_app()?.vibe().ensure_encoding(request).await)
    }
}
