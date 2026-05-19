use std::collections::BTreeMap;

use crate::ast::ExtensionCall;
use crate::diagnostics::{PromptDiagnostic, PromptDiagnosticKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionDescriptor {
    pub name: String,
    pub min_args: usize,
    pub max_args: usize,
    pub named_args: Vec<String>,
}

impl FunctionDescriptor {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        min_args: usize,
        max_args: usize,
        named_args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            name: name.into(),
            min_args,
            max_args,
            named_args: named_args.into_iter().map(Into::into).collect(),
        }
    }

    fn accepts_named_arg(&self, name: &str) -> bool {
        self.named_args.iter().any(|item| item == name)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FunctionRegistry {
    functions: BTreeMap<String, FunctionDescriptor>,
}

impl FunctionRegistry {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            functions: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn atelier_defaults() -> Self {
        Self::from_descriptors([
            FunctionDescriptor::new("chunk", 1, 1, ["name"]),
            FunctionDescriptor::new("preset", 1, 1, ["name"]),
        ])
    }

    #[must_use]
    pub fn from_descriptors(descriptors: impl IntoIterator<Item = FunctionDescriptor>) -> Self {
        let functions = descriptors
            .into_iter()
            .map(|descriptor| (descriptor.name.clone(), descriptor))
            .collect();
        Self { functions }
    }

    #[must_use]
    pub fn validate_call(&self, call: &ExtensionCall) -> Vec<PromptDiagnostic> {
        if self.functions.is_empty() {
            return Vec::new();
        }
        let Some(descriptor) = self.functions.get(&call.name) else {
            return vec![PromptDiagnostic::new(
                PromptDiagnosticKind::UnknownFunction,
                call.span,
                format!("unknown extension function `{}`", call.name),
            )];
        };

        let mut diagnostics = Vec::new();
        if call.args.len() < descriptor.min_args || call.args.len() > descriptor.max_args {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::InvalidFunctionArity,
                call.span,
                format!(
                    "`{}` expects {}..={} argument(s)",
                    call.name, descriptor.min_args, descriptor.max_args
                ),
            ));
        }
        for arg in &call.args {
            if let Some(name) = &arg.name
                && !descriptor.accepts_named_arg(name)
            {
                diagnostics.push(PromptDiagnostic::new(
                    PromptDiagnosticKind::InvalidFunctionArgument,
                    call.span,
                    format!("`{}` does not accept named argument `{name}`", call.name),
                ));
            }
        }
        diagnostics
    }
}
