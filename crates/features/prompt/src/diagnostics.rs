use crate::ast::FunctionValue;
use crate::dialect::{PromptCapability, PromptSyntaxProfile};
use crate::functions::FunctionRegistry;
use crate::syntax::{ParsedPrompt, PromptSpan, PromptToken, PromptTokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PromptDiagnosticKind {
    UnclosedStrengthening,
    UnclosedWeakening,
    UnmatchedStrengtheningClose,
    UnmatchedWeakeningClose,
    UnclosedNumericEmphasis,
    UnclosedRandomizer,
    EmptyRandomizerOption,
    UnclosedFunctionCall,
    InvalidNumericWeight,
    UnterminatedString,
    UnsupportedCapability,
    AmbiguousPipe,
    UnknownFunction,
    InvalidFunctionArity,
    InvalidFunctionArgument,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptDiagnostic {
    pub kind: PromptDiagnosticKind,
    pub span: PromptSpan,
    pub message: String,
    pub capability: Option<PromptCapability>,
}

impl PromptDiagnostic {
    #[must_use]
    pub fn new(kind: PromptDiagnosticKind, span: PromptSpan, message: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            message: message.into(),
            capability: None,
        }
    }

    #[must_use]
    pub const fn with_capability(mut self, capability: PromptCapability) -> Self {
        self.capability = Some(capability);
        self
    }
}

pub fn diagnose(
    parsed: &ParsedPrompt,
    profile: &PromptSyntaxProfile,
    functions: &FunctionRegistry,
) -> Vec<PromptDiagnostic> {
    let mut diagnostics = Vec::new();
    let ast = parsed.ast();
    let protected_ranges = crate::ast::extension_call_ranges(parsed.tokens());

    diagnose_delimiters(parsed.tokens(), &protected_ranges, &mut diagnostics);
    diagnose_unterminated_strings(parsed.tokens(), &protected_ranges, &mut diagnostics);
    diagnose_numeric_emphasis(&ast, profile, &mut diagnostics);
    diagnose_randomizers(&ast, profile, &mut diagnostics);
    diagnose_pipes(&ast, profile, &mut diagnostics);
    diagnose_extension_calls(&ast, functions, &mut diagnostics);

    diagnostics
}

fn diagnose_delimiters(
    tokens: &[PromptToken],
    protected_ranges: &[(usize, usize, bool)],
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    diagnose_unclosed_pairs(
        tokens,
        PromptTokenKind::LBrace,
        PromptTokenKind::RBrace,
        PromptDiagnosticKind::UnclosedStrengthening,
        "strengthening block is not closed",
        protected_ranges,
        diagnostics,
    );
    diagnose_unmatched_closers(
        tokens,
        PromptTokenKind::LBrace,
        PromptTokenKind::RBrace,
        PromptDiagnosticKind::UnmatchedStrengtheningClose,
        "strengthening close delimiter has no matching opener",
        protected_ranges,
        diagnostics,
    );
    diagnose_unmatched_closers(
        tokens,
        PromptTokenKind::LBracket,
        PromptTokenKind::RBracket,
        PromptDiagnosticKind::UnmatchedWeakeningClose,
        "weakening close delimiter has no matching opener",
        protected_ranges,
        diagnostics,
    );
    diagnose_unclosed_pairs(
        tokens,
        PromptTokenKind::LBracket,
        PromptTokenKind::RBracket,
        PromptDiagnosticKind::UnclosedWeakening,
        "weakening block is not closed",
        protected_ranges,
        diagnostics,
    );
}

fn diagnose_unterminated_strings(
    tokens: &[PromptToken],
    protected_ranges: &[(usize, usize, bool)],
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    diagnostics.extend(
        tokens
            .iter()
            .enumerate()
            .filter(|(index, _)| !crate::ast::is_inside_range(*index, protected_ranges))
            .map(|(_, token)| token)
            .filter(|token| token.kind == PromptTokenKind::UnterminatedString)
            .map(|token| {
                PromptDiagnostic::new(
                    PromptDiagnosticKind::UnterminatedString,
                    token.span,
                    "string literal is not closed",
                )
            }),
    );
}

fn diagnose_numeric_emphasis(
    ast: &crate::ast::PromptAst,
    profile: &PromptSyntaxProfile,
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    for numeric in ast.numeric_emphasis() {
        if !numeric.valid_weight {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::InvalidNumericWeight,
                numeric.span,
                "numeric emphasis weight is invalid",
            ));
        }
        if !numeric.closed {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::UnclosedNumericEmphasis,
                numeric.span,
                "numeric emphasis block is not closed",
            ));
        }
        if !profile.supports(PromptCapability::NumericEmphasis) {
            diagnostics.push(
                PromptDiagnostic::new(
                    PromptDiagnosticKind::UnsupportedCapability,
                    numeric.span,
                    "numeric emphasis is not supported by this syntax profile",
                )
                .with_capability(PromptCapability::NumericEmphasis),
            );
        }
        if numeric.is_negative && !profile.supports(PromptCapability::NegativeNumericEmphasis) {
            diagnostics.push(
                PromptDiagnostic::new(
                    PromptDiagnosticKind::UnsupportedCapability,
                    numeric.span,
                    "negative numeric emphasis is not supported by this syntax profile",
                )
                .with_capability(PromptCapability::NegativeNumericEmphasis),
            );
        }
    }
}

fn diagnose_randomizers(
    ast: &crate::ast::PromptAst,
    profile: &PromptSyntaxProfile,
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    for randomizer in ast.randomizers() {
        if !profile.supports(PromptCapability::Randomizer) {
            diagnostics.push(
                PromptDiagnostic::new(
                    PromptDiagnosticKind::UnsupportedCapability,
                    randomizer.span,
                    "prompt randomizer is not supported by this syntax profile",
                )
                .with_capability(PromptCapability::Randomizer),
            );
        }
        if !randomizer.closed {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::UnclosedRandomizer,
                randomizer.span,
                "prompt randomizer is not closed",
            ));
        }
        if randomizer
            .options
            .iter()
            .any(|option| option.trim().is_empty())
        {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::EmptyRandomizerOption,
                randomizer.span,
                "prompt randomizer contains an empty option",
            ));
        }
    }
}

fn diagnose_unmatched_closers(
    tokens: &[PromptToken],
    open: PromptTokenKind,
    close: PromptTokenKind,
    kind: PromptDiagnosticKind,
    message: &str,
    protected_ranges: &[(usize, usize, bool)],
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if crate::ast::is_inside_range(index, protected_ranges) {
            continue;
        }
        if token.kind == open {
            depth += 1;
        } else if token.kind == close {
            if depth == 0 {
                diagnostics.push(PromptDiagnostic::new(kind, token.span, message));
            } else {
                depth -= 1;
            }
        }
    }
}

fn diagnose_pipes(
    ast: &crate::ast::PromptAst,
    profile: &PromptSyntaxProfile,
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    for pipe in ast.pipes() {
        if !profile.supports(PromptCapability::MultiCharacterPipe)
            && !profile.supports(PromptCapability::PromptMixingPipe)
        {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::AmbiguousPipe,
                pipe.span,
                "pipe cannot be interpreted by this syntax profile",
            ));
        }
    }
}

fn diagnose_extension_calls(
    ast: &crate::ast::PromptAst,
    functions: &FunctionRegistry,
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    for call in ast.extension_calls() {
        if !call.closed {
            diagnostics.push(PromptDiagnostic::new(
                PromptDiagnosticKind::UnclosedFunctionCall,
                call.span,
                "extension function call is not closed",
            ));
        }
        for arg in &call.args {
            if matches!(arg.value, FunctionValue::InvalidString(_)) {
                diagnostics.push(PromptDiagnostic::new(
                    PromptDiagnosticKind::UnterminatedString,
                    call.span,
                    "extension function argument string is not closed",
                ));
            }
        }
        diagnostics.extend(functions.validate_call(call));
    }
}

fn diagnose_unclosed_pairs(
    tokens: &[PromptToken],
    open: PromptTokenKind,
    close: PromptTokenKind,
    kind: PromptDiagnosticKind,
    message: &str,
    protected_ranges: &[(usize, usize, bool)],
    diagnostics: &mut Vec<PromptDiagnostic>,
) {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if crate::ast::is_inside_range(index, protected_ranges) {
            continue;
        }
        if token.kind == open {
            stack.push(token.span);
        } else if token.kind == close {
            let _matched = stack.pop();
        }
    }
    diagnostics.extend(
        stack
            .into_iter()
            .map(|span| PromptDiagnostic::new(kind, span, message)),
    );
}
