use crate::syntax::{PromptSpan, PromptToken, PromptTokenKind};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PromptAst {
    strengthening: Vec<PromptSpan>,
    weakening: Vec<PromptSpan>,
    numeric_emphasis: Vec<NumericEmphasis>,
    randomizers: Vec<Randomizer>,
    extension_calls: Vec<ExtensionCall>,
    pipes: Vec<Pipe>,
}

impl PromptAst {
    #[must_use]
    pub fn from_tokens(tokens: &[PromptToken]) -> Self {
        let extension_call_ranges = extension_call_ranges(tokens);
        let randomizer_ranges = randomizer_ranges(tokens, &extension_call_ranges);
        Self {
            strengthening: paired_spans(
                tokens,
                PromptTokenKind::LBrace,
                PromptTokenKind::RBrace,
                &extension_call_ranges,
            ),
            weakening: paired_spans(
                tokens,
                PromptTokenKind::LBracket,
                PromptTokenKind::RBracket,
                &extension_call_ranges,
            ),
            numeric_emphasis: numeric_emphasis(tokens, &extension_call_ranges),
            randomizers: randomizers(tokens, &randomizer_ranges, &extension_call_ranges),
            extension_calls: extension_calls(tokens),
            pipes: pipes(tokens, &randomizer_ranges, &extension_call_ranges),
        }
    }

    #[must_use]
    pub fn strengthening(&self) -> &[PromptSpan] {
        &self.strengthening
    }

    #[must_use]
    pub fn weakening(&self) -> &[PromptSpan] {
        &self.weakening
    }

    #[must_use]
    pub fn numeric_emphasis(&self) -> &[NumericEmphasis] {
        &self.numeric_emphasis
    }

    #[must_use]
    pub fn randomizers(&self) -> &[Randomizer] {
        &self.randomizers
    }

    #[must_use]
    pub fn extension_calls(&self) -> &[ExtensionCall] {
        &self.extension_calls
    }

    #[must_use]
    pub fn pipes(&self) -> &[Pipe] {
        &self.pipes
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumericEmphasis {
    pub weight: String,
    pub span: PromptSpan,
    pub closed: bool,
    pub valid_weight: bool,
    pub is_negative: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Randomizer {
    pub span: PromptSpan,
    pub options: Vec<String>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pipe {
    pub span: PromptSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionCall {
    pub name: String,
    pub span: PromptSpan,
    pub args: Vec<FunctionArg>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionArg {
    pub name: Option<String>,
    pub value: FunctionValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionValue {
    String(String),
    Number(String),
    Identifier(String),
    Raw(String),
    InvalidString(String),
}

fn paired_spans(
    tokens: &[PromptToken],
    open: PromptTokenKind,
    close: PromptTokenKind,
    protected_ranges: &[(usize, usize, bool)],
) -> Vec<PromptSpan> {
    let mut stack = Vec::new();
    let mut spans = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if is_inside_range(index, protected_ranges) {
            continue;
        }
        if token.kind == open {
            stack.push(token.span.start);
        } else if token.kind == close
            && let Some(start) = stack.pop()
        {
            spans.push(PromptSpan::new(start, token.span.end));
        }
    }
    spans
}

fn numeric_emphasis(
    tokens: &[PromptToken],
    protected_ranges: &[(usize, usize, bool)],
) -> Vec<NumericEmphasis> {
    let mut items = Vec::new();
    let mut index = 0;
    while index + 1 < tokens.len() {
        if is_inside_range(index, protected_ranges) {
            index += 1;
            continue;
        }
        let token = &tokens[index];
        if !matches!(
            token.kind,
            PromptTokenKind::Number | PromptTokenKind::InvalidNumber
        ) || tokens[index + 1].kind != PromptTokenKind::DoubleColon
        {
            index += 1;
            continue;
        }
        let close = (index + 2..tokens.len()).find(|candidate| {
            tokens[*candidate].kind == PromptTokenKind::DoubleColon
                && !is_inside_range(*candidate, protected_ranges)
        });
        let end = close.map_or(token.span.end, |close_index| tokens[close_index].span.end);
        items.push(NumericEmphasis {
            weight: token.text.clone(),
            span: PromptSpan::new(token.span.start, end),
            closed: close.is_some(),
            valid_weight: token.kind == PromptTokenKind::Number,
            is_negative: token.text.starts_with('-'),
        });
        index = close.map_or(index + 2, |close_index| close_index + 1);
    }
    items
}

fn randomizer_ranges(
    tokens: &[PromptToken],
    protected_ranges: &[(usize, usize, bool)],
) -> Vec<(usize, usize, bool)> {
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index].kind != PromptTokenKind::DoublePipe
            || is_inside_range(index, protected_ranges)
        {
            index += 1;
            continue;
        }
        let close = (index + 1..tokens.len()).find(|candidate| {
            tokens[*candidate].kind == PromptTokenKind::DoublePipe
                && !is_inside_range(*candidate, protected_ranges)
        });
        let end = close.unwrap_or_else(|| tokens.len().saturating_sub(1));
        ranges.push((index, end, close.is_some()));
        index = close.map_or(tokens.len(), |close_index| close_index + 1);
    }
    ranges
}

fn randomizers(
    tokens: &[PromptToken],
    ranges: &[(usize, usize, bool)],
    protected_ranges: &[(usize, usize, bool)],
) -> Vec<Randomizer> {
    ranges
        .iter()
        .filter_map(|(start, end, closed)| {
            let first = tokens.get(*start)?;
            let last = tokens.get(*end)?;
            let option_end = if *closed { *end } else { end.saturating_add(1) };
            Some(Randomizer {
                span: PromptSpan::new(first.span.start, last.span.end),
                options: randomizer_options(tokens, start + 1, option_end, protected_ranges),
                closed: *closed,
            })
        })
        .collect()
}

fn randomizer_options(
    tokens: &[PromptToken],
    start: usize,
    end: usize,
    protected_ranges: &[(usize, usize, bool)],
) -> Vec<String> {
    let mut options = vec![String::new()];
    for (index, token) in tokens.iter().enumerate().take(end).skip(start) {
        if token.kind == PromptTokenKind::Pipe && !is_inside_range(index, protected_ranges) {
            options.push(String::new());
        } else if let Some(last) = options.last_mut() {
            last.push_str(&token.text);
        }
    }
    options
}

fn pipes(
    tokens: &[PromptToken],
    randomizer_ranges: &[(usize, usize, bool)],
    extension_call_ranges: &[(usize, usize, bool)],
) -> Vec<Pipe> {
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.kind == PromptTokenKind::Pipe
                && !is_inside_range(*index, randomizer_ranges)
                && !is_inside_range(*index, extension_call_ranges)
        })
        .map(|(_, token)| Pipe { span: token.span })
        .collect()
}

pub fn is_inside_range(index: usize, ranges: &[(usize, usize, bool)]) -> bool {
    ranges
        .iter()
        .any(|(start, end, closed)| index > *start && (index < *end || (!closed && index <= *end)))
}

pub fn extension_call_ranges(tokens: &[PromptToken]) -> Vec<(usize, usize, bool)> {
    let mut ranges = Vec::new();
    let mut index = 0;
    while index + 2 < tokens.len() {
        if tokens[index].kind != PromptTokenKind::At
            || tokens[index + 1].kind != PromptTokenKind::Identifier
            || tokens[index + 2].kind != PromptTokenKind::LParen
        {
            index += 1;
            continue;
        }
        let close = (index + 3..tokens.len())
            .find(|candidate| tokens[*candidate].kind == PromptTokenKind::RParen);
        ranges.push((
            index,
            close.unwrap_or_else(|| tokens.len().saturating_sub(1)),
            close.is_some(),
        ));
        index = close.map_or(tokens.len(), |close_index| close_index + 1);
    }
    ranges
}

fn extension_calls(tokens: &[PromptToken]) -> Vec<ExtensionCall> {
    let mut calls = Vec::new();
    let mut index = 0;
    while index + 2 < tokens.len() {
        if tokens[index].kind != PromptTokenKind::At
            || tokens[index + 1].kind != PromptTokenKind::Identifier
            || tokens[index + 2].kind != PromptTokenKind::LParen
        {
            index += 1;
            continue;
        }
        let close = (index + 3..tokens.len())
            .find(|candidate| tokens[*candidate].kind == PromptTokenKind::RParen);
        let args_end = close.unwrap_or(tokens.len());
        let span_end = close
            .and_then(|close_index| tokens.get(close_index))
            .or_else(|| tokens.last())
            .map_or(tokens[index + 2].span.end, |token| token.span.end);
        calls.push(ExtensionCall {
            name: tokens[index + 1].text.clone(),
            span: PromptSpan::new(tokens[index].span.start, span_end),
            args: parse_args(&tokens[index + 3..args_end]),
            closed: close.is_some(),
        });
        index = close.map_or(tokens.len(), |close_index| close_index + 1);
    }
    calls
}

fn parse_args(tokens: &[PromptToken]) -> Vec<FunctionArg> {
    split_args(tokens)
        .into_iter()
        .filter_map(parse_arg)
        .collect()
}

fn split_args(tokens: &[PromptToken]) -> Vec<&[PromptToken]> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == PromptTokenKind::Comma {
            parts.push(&tokens[start..index]);
            start = index + 1;
        }
    }
    parts.push(&tokens[start..]);
    parts
}

fn parse_arg(tokens: &[PromptToken]) -> Option<FunctionArg> {
    let tokens = trim_whitespace(tokens);
    if tokens.is_empty() {
        return None;
    }
    if tokens
        .first()
        .is_some_and(|token| token.kind == PromptTokenKind::Identifier)
        && let Some(equals_index) = named_arg_equals_index(tokens)
    {
        return Some(FunctionArg {
            name: Some(tokens[0].text.clone()),
            value: value_from_tokens(trim_whitespace(&tokens[equals_index + 1..])),
        });
    }
    Some(FunctionArg {
        name: None,
        value: value_from_tokens(tokens),
    })
}

fn named_arg_equals_index(tokens: &[PromptToken]) -> Option<usize> {
    tokens
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, token)| token.kind != PromptTokenKind::Whitespace)
        .and_then(|(index, token)| (token.kind == PromptTokenKind::Equals).then_some(index))
}

fn trim_whitespace(tokens: &[PromptToken]) -> &[PromptToken] {
    let start = tokens
        .iter()
        .position(|token| token.kind != PromptTokenKind::Whitespace)
        .unwrap_or(tokens.len());
    let end = tokens
        .iter()
        .rposition(|token| token.kind != PromptTokenKind::Whitespace)
        .map_or(start, |index| index + 1);
    &tokens[start..end]
}

fn value_from_tokens(tokens: &[PromptToken]) -> FunctionValue {
    if tokens.len() == 1 {
        let token = &tokens[0];
        return match token.kind {
            PromptTokenKind::String => FunctionValue::String(unquote_string(&token.text)),
            PromptTokenKind::UnterminatedString => FunctionValue::InvalidString(token.text.clone()),
            PromptTokenKind::Number => FunctionValue::Number(token.text.clone()),
            PromptTokenKind::Identifier => FunctionValue::Identifier(token.text.clone()),
            _ => FunctionValue::Raw(token.text.clone()),
        };
    }
    FunctionValue::Raw(tokens.iter().map(|token| token.text.as_str()).collect())
}

fn unquote_string(value: &str) -> String {
    let inner = value
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'));
    inner
        .unwrap_or(value)
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}
