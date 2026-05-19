use std::future::Future;
use std::pin::Pin;

use nai_atelier_prompt::{ExtensionCall, parse_prompt};

use crate::functions::{PromptFunctionContext, PromptFunctionRegistry, PromptFunctionTraceEntry};
use crate::text::{ExpandedPromptFragment, render_expanded_prompt_fragments};
use crate::{PromptResourceError, PromptResourceReader, PromptResourceResult};

const DEFAULT_MAX_EXPANSION_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilePromptRequest {
    pub prompt: String,
    pub max_depth: usize,
}

impl CompilePromptRequest {
    #[must_use]
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            max_depth: DEFAULT_MAX_EXPANSION_DEPTH,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledPrompt {
    pub expanded_prompt: String,
    pub trace: PromptTrace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptTrace {
    pub raw_prompt: String,
    pub expanded_prompt: String,
    pub function_calls: Vec<PromptFunctionTraceEntry>,
}

pub struct PromptCompiler<R> {
    repository: R,
    functions: PromptFunctionRegistry,
}

impl<R> PromptCompiler<R> {
    #[must_use]
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            functions: PromptFunctionRegistry::atelier_defaults(),
        }
    }

    #[must_use]
    pub const fn with_function_registry(repository: R, functions: PromptFunctionRegistry) -> Self {
        Self {
            repository,
            functions,
        }
    }
}

impl<R> PromptCompiler<R>
where
    R: PromptResourceReader,
{
    /// Expands registered prompt functions and returns the compiled prompt.
    ///
    /// # Errors
    /// Returns an error when a function is unknown, invalid, missing its target
    /// resource, exceeds maximum depth, or enters a cycle.
    pub async fn compile(
        &self,
        request: CompilePromptRequest,
    ) -> PromptResourceResult<CompiledPrompt> {
        let mut trace = Vec::new();
        let expanded_prompt = self
            .expand_text(
                &request.prompt,
                0,
                request.max_depth,
                &mut Vec::new(),
                &mut trace,
            )
            .await?;
        Ok(CompiledPrompt {
            expanded_prompt: expanded_prompt.clone(),
            trace: PromptTrace {
                raw_prompt: request.prompt,
                expanded_prompt,
                function_calls: trace,
            },
        })
    }

    fn expand_text<'a>(
        &'a self,
        text: &'a str,
        depth: usize,
        max_depth: usize,
        active_scopes: &'a mut Vec<String>,
        trace: &'a mut Vec<PromptFunctionTraceEntry>,
    ) -> Pin<Box<dyn Future<Output = PromptResourceResult<String>> + Send + 'a>> {
        Box::pin(async move {
            let mut current = text.to_owned();
            loop {
                let expanded = self
                    .expand_one_pass(&current, depth, max_depth, active_scopes, trace)
                    .await?;
                if expanded == current {
                    return Ok(expanded);
                }
                current = expanded;
            }
        })
    }

    async fn expand_one_pass(
        &self,
        text: &str,
        depth: usize,
        max_depth: usize,
        active_scopes: &mut Vec<String>,
        trace: &mut Vec<PromptFunctionTraceEntry>,
    ) -> PromptResourceResult<String> {
        let parsed = parse_prompt(text);
        let ast = parsed.ast();
        if ast.extension_calls().is_empty() {
            return Ok(text.to_owned());
        }

        let mut cursor = 0;
        let mut fragments = Vec::new();
        for call in ast.extension_calls() {
            fragments.push(ExpandedPromptFragment::text(&text[cursor..call.span.start]));
            let replacement = self
                .expand_call(call, text, depth, max_depth, active_scopes, trace)
                .await?;
            fragments.push(ExpandedPromptFragment::expansion(replacement));
            cursor = call.span.end;
        }
        fragments.push(ExpandedPromptFragment::text(&text[cursor..]));
        Ok(render_expanded_prompt_fragments(fragments))
    }

    async fn expand_call(
        &self,
        call: &ExtensionCall,
        source: &str,
        depth: usize,
        max_depth: usize,
        active_scopes: &mut Vec<String>,
        trace: &mut Vec<PromptFunctionTraceEntry>,
    ) -> PromptResourceResult<String> {
        let call_depth = depth + 1;
        if call_depth > max_depth {
            return Err(PromptResourceError::conflict(
                "prompt function expansion depth exceeded",
            ));
        }
        let function = self.functions.get(&call.name)?;
        let context = PromptFunctionContext {
            resources: &self.repository,
        };
        let output = function.execute(call, &context).await?;
        if let Some(scope) = &output.scope
            && active_scopes.iter().any(|item| item == scope)
        {
            let mut call_chain = active_scopes.clone();
            call_chain.push(scope.clone());
            return Err(PromptResourceError::cycle_detected(call_chain));
        }
        let raw_call = source[call.span.start..call.span.end].to_owned();
        let resolved_arguments = function.resolved_arguments(call)?;
        trace.push(PromptFunctionTraceEntry {
            function_name: function.descriptor().name.clone(),
            raw_call,
            resolved_arguments,
            result_text: output.text.clone(),
            depth: call_depth,
            call_chain: active_scopes.clone(),
        });
        let Some(text) = output.text else {
            return Ok(String::new());
        };
        if let Some(scope) = output.scope {
            active_scopes.push(scope);
            let expanded = self
                .expand_text(&text, call_depth, max_depth, active_scopes, trace)
                .await;
            active_scopes.pop();
            expanded
        } else {
            self.expand_text(&text, call_depth, max_depth, active_scopes, trace)
                .await
        }
    }
}
