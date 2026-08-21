use atelier_prompt::{ExtensionCall, FunctionValue, parse_prompt};

use crate::PromptChunkKey;

#[must_use]
pub fn chunk_references_in_text(text: &str, key: &PromptChunkKey) -> bool {
    let parsed = parse_prompt(text);
    parsed
        .ast()
        .extension_calls()
        .iter()
        .any(|call| chunk_call_key(call).is_some_and(|value| value == key.as_str()))
}

#[must_use]
pub fn rewrite_chunk_references(
    text: &str,
    old_key: &PromptChunkKey,
    new_key: &PromptChunkKey,
) -> String {
    let parsed = parse_prompt(text);
    let ast = parsed.ast();
    let mut output = String::new();
    let mut cursor = 0;
    for call in ast.extension_calls() {
        if chunk_call_key(call) != Some(old_key.as_str()) {
            continue;
        }
        output.push_str(&text[cursor..call.span.start]);
        output.push_str("$chunk(");
        output.push_str(new_key.as_str());
        output.push(')');
        cursor = call.span.end;
    }
    if cursor == 0 {
        text.to_owned()
    } else {
        output.push_str(&text[cursor..]);
        output
    }
}

pub fn chunk_call_key(call: &ExtensionCall) -> Option<&str> {
    if call.name != "chunk" || !call.closed || call.args.len() != 1 {
        return None;
    }
    let arg = &call.args[0];
    if arg.name.is_some() {
        return None;
    }
    match &arg.value {
        FunctionValue::Identifier(value) => Some(value.as_str()),
        _ => None,
    }
}

pub fn chunk_reference_keys_in_text(text: &str) -> Vec<PromptChunkKey> {
    parse_prompt(text)
        .ast()
        .extension_calls()
        .iter()
        .filter_map(chunk_call_key)
        .filter_map(|key| PromptChunkKey::parse(key).ok())
        .collect()
}
