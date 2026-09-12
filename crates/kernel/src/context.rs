use crate::{
    EnsureVibeEncoding, EnsuredVibeEncoding, ExportVibeDocument, ExportedVibeDocument,
    ImportEmbeddedPngVibeDocument, ImportVibeDocument, ImportedVibeDocuments, KernelClock,
    KernelEvent, KernelEventKind, KernelEventSink, KernelOutputPorts, KernelResult,
    KernelVibePorts, RanDirectorTool, RunDirectorTool,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Shared workflow services and event identity, independent of generation queue state.
#[derive(Debug)]
pub struct WorkflowContext<P> {
    ports: Arc<P>,
    sequence: Arc<AtomicU64>,
}

impl<P> WorkflowContext<P> {
    #[must_use]
    pub fn new(ports: P) -> Self {
        Self {
            ports: Arc::new(ports),
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }
    #[must_use]
    pub fn ports(&self) -> &P {
        &self.ports
    }
}
impl<P: KernelEventSink> WorkflowContext<P> {
    pub(crate) async fn emit(&self, kind: KernelEventKind) {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
        self.ports.emit(KernelEvent { sequence, kind }).await;
    }
}
impl<P> WorkflowContext<P>
where
    P: KernelClock + KernelOutputPorts + atelier_director::NovelAiDirectorClient + KernelEventSink,
{
    /// Runs one Director tool request and indexes the produced image.
    ///
    /// # Errors
    /// Returns an error when the Director client fails or persistence/indexing
    /// cannot complete.
    pub async fn run_director_tool(
        &self,
        request: RunDirectorTool,
    ) -> KernelResult<RanDirectorTool> {
        crate::workflow::director::run_director_tool(self, request).await
    }
}

impl<P> WorkflowContext<P>
where
    P: KernelVibePorts,
{
    /// Imports official Vibe JSON and registers its document resources.
    ///
    /// # Errors
    /// Returns an error when the document is invalid, resource registration
    /// fails, or repository persistence fails.
    pub async fn import_vibe_document(
        &self,
        request: ImportVibeDocument,
    ) -> KernelResult<ImportedVibeDocuments> {
        crate::workflow::vibe::import_vibe_document(self, request).await
    }

    /// Extracts an embedded Vibe document from PNG bytes and imports it.
    ///
    /// # Errors
    /// Returns an error when extraction, import, resource registration, or
    /// repository persistence fails.
    pub async fn import_embedded_png_vibe_document(
        &self,
        request: ImportEmbeddedPngVibeDocument,
    ) -> KernelResult<ImportedVibeDocuments> {
        crate::workflow::vibe::import_embedded_png_vibe_document(self, request).await
    }

    /// Exports one or more managed Vibes as official JSON.
    ///
    /// # Errors
    /// Returns an error when a Vibe cannot be found, its document resource
    /// cannot be read, or the requested format is invalid for the selection.
    pub async fn export_vibe_document(
        &self,
        request: ExportVibeDocument,
    ) -> KernelResult<ExportedVibeDocument> {
        crate::workflow::vibe::export_vibe_document(self, request).await
    }

    /// Ensures a model/settings-specific Vibe encoding exists for a source image.
    ///
    /// # Errors
    /// Returns an error when cache lookup, `NovelAI` encoding, resource
    /// registration, or cache persistence fails.
    pub async fn ensure_vibe_encoding(
        &self,
        request: EnsureVibeEncoding,
    ) -> KernelResult<EnsuredVibeEncoding> {
        crate::workflow::vibe::ensure_vibe_encoding(self, request).await
    }
}

impl<P> Clone for WorkflowContext<P> {
    fn clone(&self) -> Self {
        Self {
            ports: self.ports.clone(),
            sequence: self.sequence.clone(),
        }
    }
}
