use super::{CompiledPrompt, Deserialize, PromptFunctionTraceEntry, PromptTrace, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct CompiledPromptDto {
    expanded_prompt: String,
    trace: PromptTraceDto,
}

impl From<&CompiledPrompt> for CompiledPromptDto {
    fn from(value: &CompiledPrompt) -> Self {
        Self {
            expanded_prompt: value.expanded_prompt.clone(),
            trace: PromptTraceDto::from(&value.trace),
        }
    }
}

impl CompiledPromptDto {
    pub(super) fn into_domain(self) -> CompiledPrompt {
        CompiledPrompt {
            expanded_prompt: self.expanded_prompt,
            trace: self.trace.into_domain(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct PromptTraceDto {
    raw_prompt: String,
    expanded_prompt: String,
    function_calls: Vec<PromptFunctionTraceEntryDto>,
}

impl From<&PromptTrace> for PromptTraceDto {
    fn from(value: &PromptTrace) -> Self {
        Self {
            raw_prompt: value.raw_prompt.clone(),
            expanded_prompt: value.expanded_prompt.clone(),
            function_calls: value
                .function_calls
                .iter()
                .map(PromptFunctionTraceEntryDto::from)
                .collect(),
        }
    }
}

impl PromptTraceDto {
    pub(super) fn into_domain(self) -> PromptTrace {
        PromptTrace {
            raw_prompt: self.raw_prompt,
            expanded_prompt: self.expanded_prompt,
            function_calls: self
                .function_calls
                .into_iter()
                .map(PromptFunctionTraceEntryDto::into_domain)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct PromptFunctionTraceEntryDto {
    function_name: String,
    raw_call: String,
    resolved_arguments: Vec<String>,
    result_text: Option<String>,
    depth: usize,
    call_chain: Vec<String>,
}

impl From<&PromptFunctionTraceEntry> for PromptFunctionTraceEntryDto {
    fn from(value: &PromptFunctionTraceEntry) -> Self {
        Self {
            function_name: value.function_name.clone(),
            raw_call: value.raw_call.clone(),
            resolved_arguments: value.resolved_arguments.clone(),
            result_text: value.result_text.clone(),
            depth: value.depth,
            call_chain: value.call_chain.clone(),
        }
    }
}

impl PromptFunctionTraceEntryDto {
    pub(super) fn into_domain(self) -> PromptFunctionTraceEntry {
        PromptFunctionTraceEntry {
            function_name: self.function_name,
            raw_call: self.raw_call,
            resolved_arguments: self.resolved_arguments,
            result_text: self.result_text,
            depth: self.depth,
            call_chain: self.call_chain,
        }
    }
}
