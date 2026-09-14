use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum TextEditError {
    #[error("empty_match: old_text must not be empty")]
    EmptyMatch,
    #[error("text_not_found: old_text does not occur in the target")]
    NotFound,
    #[error(
        "ambiguous_match: old_text occurs {0} times; include more context or explicitly replace all"
    )]
    Ambiguous(usize),
}

/// Replaces an exact literal without normalizing prompt syntax or whitespace.
///
/// # Errors
/// Rejects an empty match, a missing match, or multiple matches unless `all` is explicit.
pub fn replace_prompt_text(
    text: &str,
    old_text: &str,
    new_text: &str,
    all: bool,
) -> Result<String, TextEditError> {
    if old_text.is_empty() {
        return Err(TextEditError::EmptyMatch);
    }
    let count = text
        .char_indices()
        .filter(|(index, _)| text[*index..].starts_with(old_text))
        .count();
    match count {
        0 => Err(TextEditError::NotFound),
        1 => Ok(text.replacen(old_text, new_text, 1)),
        _ if all => Ok(text.replace(old_text, new_text)),
        _ => Err(TextEditError::Ambiguous(count)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_the_exact_fragment_preserving_syntax() {
        assert_eq!(
            replace_prompt_text(
                "{{红发}}, 1.2::red hair::, $chunk(style)",
                "red hair",
                "silver hair",
                false
            ),
            Ok("{{红发}}, 1.2::silver hair::, $chunk(style)".to_owned())
        );
    }

    #[test]
    fn requires_unambiguous_nonempty_match() {
        assert_eq!(
            replace_prompt_text("cat, cat", "cat", "dog", false),
            Err(TextEditError::Ambiguous(2))
        );
        assert_eq!(
            replace_prompt_text("cat", "Cat", "dog", false),
            Err(TextEditError::NotFound)
        );
        assert_eq!(
            replace_prompt_text("cat", "", "dog", true),
            Err(TextEditError::EmptyMatch)
        );
        assert_eq!(
            replace_prompt_text("cat, cat", "cat", "dog", true),
            Ok("dog, dog".to_owned())
        );
        assert_eq!(
            replace_prompt_text("cat", "cat", "", false),
            Ok(String::new())
        );
    }
}
