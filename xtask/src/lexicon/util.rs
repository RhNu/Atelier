use super::{Path, SystemTime, UNIX_EPOCH, UnicodeNormalization, fs};

pub(super) fn classify_other_bucket(tag: &str) -> String {
    let Some(first) = tag.trim().chars().next() else {
        return "#".to_owned();
    };
    let upper = first.to_ascii_uppercase();
    if upper.is_ascii_alphabetic() {
        upper.to_string()
    } else {
        "#".to_owned()
    }
}

pub(super) fn is_valid_tag(tag: &str) -> bool {
    let trimmed = tag.trim();
    !normalize_tag(trimmed).is_empty() && trimmed.is_ascii()
}

pub(super) fn normalize_tag_display(tag: &str) -> String {
    tag.trim().to_owned()
}

pub(super) fn normalize_tag(tag: &str) -> String {
    normalize_comparable_text(&tag.replace('_', " "))
}

pub(super) fn normalize_text(value: &str) -> String {
    normalize_comparable_text(value)
}

fn normalize_comparable_text(value: &str) -> String {
    value
        .nfkc()
        .collect::<String>()
        .trim()
        .to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn normalize_display_text(value: &str) -> String {
    value
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn read_json_value(path: &Path) -> Result<serde_json::Value, String> {
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&strip_bom(&raw)).map_err(|error| error.to_string())
}

pub(super) fn strip_bom(value: &str) -> String {
    value.strip_prefix('\u{feff}').unwrap_or(value).to_owned()
}

pub(super) fn unique_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
