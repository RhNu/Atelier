use unicode_normalization::UnicodeNormalization;

/// Produces a stable key for matching canonical tags, aliases, and prompt fragments.
#[must_use]
pub fn canonical_comparison_key(value: &str) -> String {
    value
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|character| {
            if character.is_whitespace() {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

/// Normalizes free text without erasing word boundaries used by FTS queries.
#[must_use]
pub fn normalized_search_text(value: &str) -> String {
    value
        .nfkc()
        .flat_map(char::to_lowercase)
        .map(|character| if character == '_' { ' ' } else { character })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_keys_fold_case_width_and_spaces() {
        assert_eq!(canonical_comparison_key("  Hatsune Miku "), "hatsune_miku");
        assert_eq!(canonical_comparison_key("ＡＢＣ"), "abc");
    }
}
