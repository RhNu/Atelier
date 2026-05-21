use std::collections::BTreeMap;

use async_trait::async_trait;
use atelier_prompt::{ExtensionCall, FunctionValue};

use crate::references::chunk_call_key;
use crate::{PromptResourceError, PromptResourceReader, PromptResourceResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptFunctionDescriptor {
    pub name: String,
}

impl PromptFunctionDescriptor {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptFunctionOutput {
    pub text: Option<String>,
    pub scope: Option<String>,
}

impl PromptFunctionOutput {
    #[must_use]
    pub fn text(text: impl Into<String>, scope: Option<String>) -> Self {
        Self {
            text: Some(text.into()),
            scope,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptFunctionTraceEntry {
    pub function_name: String,
    pub raw_call: String,
    pub resolved_arguments: Vec<String>,
    pub result_text: Option<String>,
    pub depth: usize,
    pub call_chain: Vec<String>,
}

pub struct PromptFunctionContext<'a> {
    pub resources: &'a dyn PromptResourceReader,
}

#[async_trait]
pub trait PromptFunction: Send + Sync {
    fn descriptor(&self) -> &PromptFunctionDescriptor;

    async fn execute(
        &self,
        call: &ExtensionCall,
        context: &PromptFunctionContext<'_>,
    ) -> PromptResourceResult<PromptFunctionOutput>;

    /// Resolves function arguments into trace-friendly values.
    ///
    /// # Errors
    /// Returns an error when the call arguments do not match this function's
    /// contract.
    fn resolved_arguments(&self, call: &ExtensionCall) -> PromptResourceResult<Vec<String>>;
}

pub struct PromptFunctionRegistry {
    functions: BTreeMap<String, Box<dyn PromptFunction>>,
}

impl PromptFunctionRegistry {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            functions: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn atelier_defaults() -> Self {
        Self::empty().with_function(Box::new(ChunkFunction::new()))
    }

    #[must_use]
    pub fn with_function(mut self, function: Box<dyn PromptFunction>) -> Self {
        self.insert(function);
        self
    }

    pub fn insert(&mut self, function: Box<dyn PromptFunction>) {
        self.functions
            .insert(function.descriptor().name.clone(), function);
    }

    /// Returns the function registered under `name`.
    ///
    /// # Errors
    /// Returns an error when no function is registered with that name.
    pub fn get(&self, name: &str) -> PromptResourceResult<&dyn PromptFunction> {
        let Some(function) = self.functions.get(name) else {
            return Err(PromptResourceError::invalid_request(format!(
                "unknown prompt function `{name}`"
            )));
        };
        Ok(function.as_ref())
    }
}

pub struct ChunkFunction {
    descriptor: PromptFunctionDescriptor,
}

impl ChunkFunction {
    pub fn new() -> Self {
        Self {
            descriptor: PromptFunctionDescriptor::new("chunk"),
        }
    }
}

#[async_trait]
impl PromptFunction for ChunkFunction {
    fn descriptor(&self) -> &PromptFunctionDescriptor {
        &self.descriptor
    }

    async fn execute(
        &self,
        call: &ExtensionCall,
        context: &PromptFunctionContext<'_>,
    ) -> PromptResourceResult<PromptFunctionOutput> {
        let key = chunk_key_argument(call)?;
        let chunk = context
            .resources
            .get_chunk_by_key(&key)
            .await?
            .ok_or_else(|| {
                PromptResourceError::not_found(format!("chunk `{}` does not exist", key.as_str()))
            })?;
        Ok(PromptFunctionOutput::text(
            chunk.content,
            Some(format!("chunk:{}", key.as_str())),
        ))
    }

    fn resolved_arguments(&self, call: &ExtensionCall) -> PromptResourceResult<Vec<String>> {
        Ok(vec![chunk_key_argument(call)?.as_str().to_owned()])
    }
}

pub fn chunk_key_argument(call: &ExtensionCall) -> PromptResourceResult<crate::PromptChunkKey> {
    if let Some(key) = chunk_call_key(call) {
        return crate::PromptChunkKey::parse(key);
    }
    let actual = call
        .args
        .first()
        .map_or("<missing>", |arg| match &arg.value {
            FunctionValue::Identifier(value)
            | FunctionValue::String(value)
            | FunctionValue::Number(value)
            | FunctionValue::Raw(value)
            | FunctionValue::InvalidString(value) => value.as_str(),
        });
    Err(PromptResourceError::invalid_request(format!(
        "`chunk` expects one identifier argument, got `{actual}`"
    )))
}
