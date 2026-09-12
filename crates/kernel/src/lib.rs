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
    SubmitGenerationBatch, SubmitGenerationBatchJob, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
pub use ports::{
    GenerationPayloadStore, KernelClock, KernelEventSink, KernelGenerationPorts, KernelOutputPorts,
    KernelVibePorts,
};
pub use runtime::{GenerationTaskCancellation, KernelRuntime};

mod context;
pub use context::WorkflowContext;

mod queue_view;
pub use queue_view::QueueView;
