use atelier_prompt::{FunctionRegistry, PromptDiagnosticKind, PromptSyntaxProfile, parse_prompt};
use serde::Deserialize;

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    profile: String,
    text: String,
    diagnostics: Vec<String>,
}

#[test]
fn rust_parser_matches_shared_syntax_corpus() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../../assets/prompt-syntax/corpus.json"))
            .expect("shared syntax corpus should be valid JSON");

    for case in corpus.cases {
        let profile = match case.profile.as_str() {
            "novelai_v3" => PromptSyntaxProfile::novelai_v3(),
            "novelai_v4" => PromptSyntaxProfile::novelai_v4(),
            _ => PromptSyntaxProfile::novelai_v45(),
        };
        let parsed = parse_prompt(&case.text);
        assert_eq!(parsed.to_lossless_text(), case.text, "{}", case.name);
        let mut actual = parsed
            .diagnostics_with_functions(&profile, &FunctionRegistry::atelier_defaults())
            .into_iter()
            .map(|diagnostic| diagnostic_code(diagnostic.kind))
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let mut expected = case
            .diagnostics
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(actual, expected, "{}", case.name);
    }
}

const fn diagnostic_code(kind: PromptDiagnosticKind) -> &'static str {
    match kind {
        PromptDiagnosticKind::UnclosedStrengthening => "unclosed_strengthening",
        PromptDiagnosticKind::UnclosedWeakening => "unclosed_weakening",
        PromptDiagnosticKind::UnmatchedStrengtheningClose => "unmatched_strengthening_close",
        PromptDiagnosticKind::UnmatchedWeakeningClose => "unmatched_weakening_close",
        PromptDiagnosticKind::UnclosedNumericEmphasis => "unclosed_numeric_emphasis",
        PromptDiagnosticKind::UnclosedRandomizer => "unclosed_randomizer",
        PromptDiagnosticKind::EmptyRandomizerOption => "empty_randomizer_option",
        PromptDiagnosticKind::UnclosedFunctionCall => "unclosed_function_call",
        PromptDiagnosticKind::InvalidNumericWeight => "invalid_numeric_weight",
        PromptDiagnosticKind::UnterminatedString => "unterminated_string",
        PromptDiagnosticKind::UnsupportedCapability => "unsupported_capability",
        PromptDiagnosticKind::AmbiguousPipe => "ambiguous_pipe",
        PromptDiagnosticKind::UnknownFunction => "unknown_function",
        PromptDiagnosticKind::InvalidFunctionArity => "invalid_function_arity",
        PromptDiagnosticKind::InvalidFunctionArgument => "invalid_function_argument",
    }
}
