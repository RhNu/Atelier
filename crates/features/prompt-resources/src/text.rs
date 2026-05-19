#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedPromptFragment {
    text: String,
    is_expansion: bool,
}

impl ExpandedPromptFragment {
    pub fn text(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            is_expansion: false,
        }
    }

    pub const fn expansion(text: String) -> Self {
        Self {
            text,
            is_expansion: true,
        }
    }
}

#[must_use]
pub fn render_expanded_prompt_fragments(
    fragments: impl IntoIterator<Item = ExpandedPromptFragment>,
) -> String {
    let mut output = String::new();
    let mut boundary_from_expansion = false;
    for fragment in fragments {
        if fragment.text.is_empty() {
            boundary_from_expansion |= fragment.is_expansion;
            continue;
        }
        append_fragment(
            &mut output,
            &fragment.text,
            boundary_from_expansion || fragment.is_expansion,
        );
        boundary_from_expansion = fragment.is_expansion;
    }
    output
}

fn append_fragment(output: &mut String, fragment: &str, normalize_boundary: bool) {
    if !normalize_boundary {
        output.push_str(fragment);
        return;
    }

    let Some(left) = significant_tail(output) else {
        push_without_leading_boundary(output, fragment);
        return;
    };
    let Some(right) = significant_head(fragment) else {
        trim_trailing_boundary(output);
        return;
    };
    if !can_normalize_boundary(left, right) {
        output.push_str(fragment);
        return;
    }

    trim_trailing_boundary(output);
    output.push_str(", ");
    output.push_str(trim_leading_boundary(fragment));
}

fn push_without_leading_boundary(output: &mut String, fragment: &str) {
    output.push_str(trim_leading_boundary(fragment));
}

const fn can_normalize_boundary(left: char, right: char) -> bool {
    !matches!(left, '{' | '[' | '(' | '|' | ':') && !matches!(right, '}' | ']' | ')' | '|' | ':')
}

fn significant_tail(text: &str) -> Option<char> {
    text.chars()
        .rev()
        .find(|ch| !ch.is_whitespace() && *ch != ',')
}

fn significant_head(text: &str) -> Option<char> {
    text.chars().find(|ch| !ch.is_whitespace() && *ch != ',')
}

fn trim_trailing_boundary(text: &mut String) {
    let trimmed_len = text
        .trim_end_matches(|ch: char| ch.is_whitespace() || ch == ',')
        .len();
    text.truncate(trimmed_len);
}

fn trim_leading_boundary(text: &str) -> &str {
    text.trim_start_matches(|ch: char| ch.is_whitespace() || ch == ',')
}
