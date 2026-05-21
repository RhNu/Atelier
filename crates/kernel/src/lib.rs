//! Runtime orchestration kernel for Atelier.

mod error;
mod event;
mod payload;
mod ports;
mod runtime;
mod workflow;

pub use error::{KernelError, KernelResult};
pub use event::{KernelEvent, KernelEventKind, KernelFailureDetail};
pub use payload::{
    EnsureVibeEncoding, EnsuredVibeEncoding, ExportVibeDocument, ExportedVibeDocument,
    GenerationWorkRequest, ImportEmbeddedPngVibeDocument, ImportVibeDocument,
    ImportedVibeDocuments, PreparedGenerationPayload, RanDirectorTool, RunDirectorTool,
    SubmitGenerationWork, SubmittedGenerationPayload,
};
pub use ports::{
    GenerationPayloadStore, KernelClock, KernelDirectorPorts, KernelEventSink,
    KernelGenerationPorts, KernelPreciseReferencePorts, KernelVibePorts,
};
pub use runtime::KernelRuntime;
