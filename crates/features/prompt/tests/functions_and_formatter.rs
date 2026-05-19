use nai_atelier_prompt::{
    FormatterOptions, FunctionArg, FunctionRegistry, FunctionValue, PromptDiagnosticKind,
    PromptSyntaxProfile, format_prompt, parse_prompt,
};

#[test]
fn parses_and_validates_atelier_extension_calls() {
    let parsed = parse_prompt(r#"@chunk("face"), @preset(name="v4")"#);
    let ast = parsed.ast();

    assert_eq!(ast.extension_calls().len(), 2);
    assert_eq!(ast.extension_calls()[0].name, "chunk");
    assert_eq!(
        ast.extension_calls()[0].args,
        vec![FunctionArg {
            name: None,
            value: FunctionValue::String("face".to_owned()),
        }]
    );
    assert_eq!(ast.extension_calls()[1].name, "preset");
    assert_eq!(
        ast.extension_calls()[1].args[0].name.as_deref(),
        Some("name")
    );

    let diagnostics = parsed.diagnostics_with_functions(
        &PromptSyntaxProfile::novelai_v45(),
        &FunctionRegistry::atelier_defaults(),
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn reports_extension_function_signature_errors() {
    let parsed = parse_prompt(r#"@unknown("x"), @chunk(), @preset(other = "v4")"#);
    let diagnostics = parsed.diagnostics_with_functions(
        &PromptSyntaxProfile::novelai_v45(),
        &FunctionRegistry::atelier_defaults(),
    );

    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::UnknownFunction)
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::InvalidFunctionArity)
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.kind == PromptDiagnosticKind::InvalidFunctionArgument)
    );
}

#[test]
fn named_function_arguments_allow_surrounding_whitespace() {
    let parsed = parse_prompt(r#"@preset(name = "v4")"#);
    let ast = parsed.ast();

    assert_eq!(
        ast.extension_calls()[0].args[0].name.as_deref(),
        Some("name")
    );
    assert_eq!(
        ast.extension_calls()[0].args[0].value,
        FunctionValue::String("v4".to_owned())
    );
    assert!(
        parsed
            .diagnostics_with_functions(
                &PromptSyntaxProfile::novelai_v45(),
                &FunctionRegistry::atelier_defaults(),
            )
            .is_empty()
    );
}

#[test]
fn conservative_formatter_round_trips_without_reordering_prompt_text() {
    let source = "1girl,{blue eyes},  ||red|blue||, @chunk(\"face\")";

    assert_eq!(
        format_prompt(source, &FormatterOptions::conservative()),
        source
    );
    assert_eq!(
        parse_prompt(source).format(&FormatterOptions::conservative()),
        source
    );
}
