use atelier_prompt::{
    PromptCapability, PromptDiagnosticKind, PromptSyntaxProfile, PromptTokenKind, parse_prompt,
};

#[test]
fn v5_natural_language_is_lossless_and_advisory() {
    let source = "一位画家（微笑）😀：soft light\nKeep (all) punctuation!? $not_a_call";
    let parsed = parse_prompt(source);
    assert_eq!(parsed.to_lossless_text(), source);
    let _diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v5());
}

#[test]
fn parses_v4_prompt_syntax_losslessly() {
    let source =
        r#"1girl, {blue eyes}, [lowres], 1.5::rain, night::, ||red|blue||, $chunk("face")"#;

    let parsed = parse_prompt(source);
    let ast = parsed.ast();

    assert_eq!(parsed.source(), source);
    assert_eq!(parsed.to_lossless_text(), source);
    assert!(
        parsed
            .tokens()
            .iter()
            .any(|token| token.kind == PromptTokenKind::LBrace)
    );
    assert!(
        parsed
            .tokens()
            .iter()
            .any(|token| token.kind == PromptTokenKind::DoublePipe)
    );
    let numeric_node = parsed
        .syntax()
        .descendants()
        .find(|node| node.kind() == atelier_prompt::PromptSyntaxKind::NumericEmphasis)
        .expect("numeric emphasis node should exist");
    assert_eq!(numeric_node.text().to_string(), "1.5::rain, night::");
    assert_eq!(ast.strengthening().len(), 1);
    assert_eq!(ast.weakening().len(), 1);
    assert_eq!(ast.numeric_emphasis().len(), 1);
    assert_eq!(ast.randomizers().len(), 1);
    assert_eq!(ast.extension_calls().len(), 1);
    assert_eq!(ast.extension_calls()[0].name, "chunk");
}

#[test]
fn keeps_numeric_prefix_tags_out_of_numeric_weight_tokens() {
    let parsed = parse_prompt("1girl, 1.5::cinematic::, $chunk(hero)");

    assert_eq!(parsed.tokens()[0].kind, PromptTokenKind::Text);
    assert_eq!(parsed.ast().numeric_emphasis().len(), 1);
    assert_eq!(parsed.ast().extension_calls()[0].name, "chunk");
}

#[test]
fn incomplete_extension_call_reports_diagnostic_without_panicking() {
    let parsed = parse_prompt("$chunk(");
    let diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());

    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnclosedFunctionCall)
    );
}

#[test]
fn unclosed_randomizer_keeps_internal_pipes_out_of_top_level_pipe_gate() {
    let parsed = parse_prompt("||red|");
    let profile = PromptSyntaxProfile::new("randomizer-only", vec![PromptCapability::Randomizer]);
    let diagnostics = parsed.diagnostics(&profile);

    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnclosedRandomizer)
    );
    assert!(
        !diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::AmbiguousPipe)
    );
}

#[test]
fn extension_call_arguments_do_not_leak_pipes_to_profile_gate() {
    let parsed = parse_prompt("$fn(a|b)");
    let profile = PromptSyntaxProfile::new("no-pipe", vec![PromptCapability::Randomizer]);
    let diagnostics = parsed.diagnostics(&profile);

    assert!(parsed.ast().pipes().is_empty());
    assert!(
        !diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::AmbiguousPipe)
    );
}

#[test]
fn extension_call_arguments_do_not_leak_native_prompt_syntax() {
    let parsed = parse_prompt("$fn({x}, 1.5::y::)");
    let diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v3());

    assert!(parsed.ast().strengthening().is_empty());
    assert!(parsed.ast().numeric_emphasis().is_empty());
    assert!(!diagnostics.iter().any(|item| {
        item.kind == PromptDiagnosticKind::UnsupportedCapability
            && item.capability == Some(PromptCapability::NumericEmphasis)
    }));
}

#[test]
fn extension_call_arguments_do_not_leak_delimiter_diagnostics() {
    let parsed = parse_prompt("$fn({x)");
    let diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());

    assert!(
        !diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnclosedStrengthening)
    );
}

#[test]
fn extension_call_unterminated_string_reports_once() {
    let parsed = parse_prompt("$fn(\"x)");
    let diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());
    let unterminated_count = diagnostics
        .iter()
        .filter(|item| item.kind == PromptDiagnosticKind::UnterminatedString)
        .count();

    assert_eq!(unterminated_count, 1);
}

#[test]
fn extension_call_cst_does_not_parse_native_prompt_nodes_inside_arguments() {
    let parsed = parse_prompt("$fn(1.5::x::)");

    assert!(
        parsed
            .syntax()
            .descendants()
            .all(|node| node.kind() != atelier_prompt::PromptSyntaxKind::NumericEmphasis)
    );
}

#[test]
fn numeric_emphasis_close_search_ignores_extension_call_arguments() {
    let parsed = parse_prompt("1.5::a $fn(::)");
    let diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());

    assert!(!parsed.ast().numeric_emphasis()[0].closed);
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnclosedNumericEmphasis)
    );
}

#[test]
fn randomizer_options_do_not_split_on_pipes_inside_extension_calls() {
    let parsed = parse_prompt("||$fn(a|b)|x||");

    assert_eq!(
        parsed.ast().randomizers()[0].options,
        vec!["$fn(a|b)".to_owned(), "x".to_owned()]
    );
}

#[test]
fn keeps_pipe_syntax_model_gated_instead_of_parser_gated() {
    let parsed = parse_prompt("base | character | other");

    assert_eq!(parsed.ast().pipes().len(), 2);

    let v45 = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());
    assert!(
        !v45.iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnsupportedCapability)
    );

    let v3 = parsed.diagnostics(&PromptSyntaxProfile::novelai_v3());
    assert!(
        !v3.iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnsupportedCapability)
    );
}

#[test]
fn reports_profile_specific_capability_diagnostics() {
    let parsed = parse_prompt("-1::hat:: | frog");
    let v4 = parsed.diagnostics(&PromptSyntaxProfile::novelai_v4());

    assert!(v4.iter().any(|item| {
        item.kind == PromptDiagnosticKind::UnsupportedCapability
            && item.capability == Some(PromptCapability::NegativeNumericEmphasis)
    }));

    let v3 = parsed.diagnostics(&PromptSyntaxProfile::novelai_v3());
    assert!(v3.iter().any(|item| {
        item.kind == PromptDiagnosticKind::UnsupportedCapability
            && item.capability == Some(PromptCapability::NumericEmphasis)
    }));

    let v45 = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());
    assert!(
        !v45.iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnsupportedCapability)
    );
}

#[test]
fn reports_unclosed_and_invalid_syntax_without_losing_text() {
    let source = r#"{{rain, 1..2::bad, ||red|blue, $chunk("face)"#;
    let parsed = parse_prompt(source);
    let diagnostics = parsed.diagnostics(&PromptSyntaxProfile::novelai_v45());

    assert_eq!(parsed.to_lossless_text(), source);
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnclosedStrengthening)
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::InvalidNumericWeight)
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnclosedRandomizer)
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnterminatedString)
    );
}
