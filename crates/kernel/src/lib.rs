//! Runtime orchestration kernel for NAI Atelier.

mod error;
mod event;
mod payload;
mod ports;
mod runtime;
mod workflow;

pub use error::{KernelError, KernelResult};
pub use event::{KernelEvent, KernelEventKind};
pub use payload::{
    GenerationWorkRequest, PreparedGenerationPayload, SubmitGenerationWork,
    SubmittedGenerationPayload,
};
pub use ports::{GenerationPayloadStore, KernelClock, KernelEventSink, KernelGenerationPorts};
pub use runtime::KernelRuntime;
