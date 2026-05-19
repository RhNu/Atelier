use crate::syntax::{ParsedPrompt, parse_prompt};

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FormatterOptions {
    preserve_source: bool,
}

impl FormatterOptions {
    #[must_use]
    pub const fn conservative() -> Self {
        Self {
            preserve_source: true,
        }
    }
}

#[must_use]
pub fn format_prompt(source: &str, options: &FormatterOptions) -> String {
    let parsed = parse_prompt(source);
    format_parsed_prompt(&parsed, *options)
}

#[must_use]
pub fn format_parsed_prompt(parsed: &ParsedPrompt, _options: FormatterOptions) -> String {
    parsed.to_lossless_text()
}
