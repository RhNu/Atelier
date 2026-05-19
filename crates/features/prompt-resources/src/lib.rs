//! Prompt resource management and prompt function compilation.

mod compiler;
mod error;
mod functions;
mod model;
mod ports;
mod references;
mod service;
mod text;

pub use compiler::{CompilePromptRequest, CompiledPrompt, PromptCompiler, PromptTrace};
pub use error::{
    PromptFunctionCycle, PromptResourceError, PromptResourceErrorKind, PromptResourceResult,
};
pub use functions::{
    PromptFunction, PromptFunctionContext, PromptFunctionDescriptor, PromptFunctionOutput,
    PromptFunctionRegistry, PromptFunctionTraceEntry,
};
pub use model::{
    ChunkReference, DeletePromptChunkResult, PromptChunk, PromptChunkId, PromptChunkKey,
    UpsertPromptChunkRequest,
};
pub use ports::{PromptResourceReader, PromptResourceRepository};
pub use references::{chunk_references_in_text, rewrite_chunk_references};
pub use service::PromptChunkService;
