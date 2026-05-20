//! Runtime orchestration kernel for NAI Atelier.

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
    ImportedVibeDocuments, PreparedGenerationPayload, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
pub use ports::{
    GenerationPayloadStore, KernelClock, KernelEventSink, KernelGenerationPorts,
    KernelPreciseReferencePorts, KernelVibePorts,
};
pub use runtime::KernelRuntime;
