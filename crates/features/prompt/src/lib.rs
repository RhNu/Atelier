//! Prompt parsing feature crate.

mod ast;
mod diagnostics;
mod dialect;
mod formatter;
mod functions;
mod syntax;

pub use ast::{
    ExtensionCall, FunctionArg, FunctionValue, NumericEmphasis, Pipe, PromptAst, Randomizer,
};
pub use diagnostics::{PromptDiagnostic, PromptDiagnosticKind};
pub use dialect::{PromptCapability, PromptSyntaxProfile};
pub use formatter::{FormatterOptions, format_prompt};
pub use functions::{FunctionDescriptor, FunctionRegistry};
pub use syntax::{
    ParsedPrompt, PromptSpan, PromptSyntaxKind, PromptSyntaxNode, PromptToken, PromptTokenKind,
    parse_prompt,
};
