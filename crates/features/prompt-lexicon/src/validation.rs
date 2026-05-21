use super::{LexiconFile, PromptLexiconError, PromptLexiconStats};

pub fn normalize_search_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_catalog_key(value: &str) -> String {
    value.trim().to_lowercase()
}

pub fn required_catalog_key(value: Option<&str>, name: &str) -> Result<String, PromptLexiconError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_catalog_key)
        .ok_or_else(|| {
            PromptLexiconError::InvalidRequest(format!(
                "prompt lexicon browse mode requires {name}"
            ))
        })
}

pub fn slice_checked<'a, T>(
    values: &'a [T],
    start: usize,
    count: usize,
    label: &str,
) -> Result<&'a [T], PromptLexiconError> {
    let end = start.checked_add(count).ok_or_else(|| {
        PromptLexiconError::InvalidPayload(format!("{label} is outside the payload"))
    })?;
    values.get(start..end).ok_or_else(|| {
        PromptLexiconError::InvalidPayload(format!("{label} is outside the payload"))
    })
}

pub fn validate_payload_ranges(payload: &LexiconFile) -> Result<(), PromptLexiconError> {
    for category in &payload.categories {
        slice_checked(
            &payload.subcategories,
            category.subcategory_start,
            category.subcategory_count,
            "category subcategory range",
        )?;
    }
    for subcategory in &payload.subcategories {
        slice_checked(
            &payload.tags,
            subcategory.tag_start,
            subcategory.tag_count,
            "subcategory tag range",
        )?;
    }
    for tag in &payload.tags {
        slice_checked(
            &payload.translations,
            tag.translation_start,
            tag.translation_count,
            "tag translation range",
        )?;
    }
    Ok(())
}

pub fn derive_stats(payload: &LexiconFile) -> Result<PromptLexiconStats, PromptLexiconError> {
    let total_tags = payload.tags.len() as u64;
    let total_translations = payload.translations.len() as u64;
    let matched_weights = payload
        .tags
        .iter()
        .filter(|tag| tag.weight.is_some())
        .count() as u64;
    let tags_with_aliases = payload
        .tags
        .iter()
        .filter(|tag| tag.translation_count > 1)
        .count() as u64;
    let max_aliases_per_tag = payload
        .tags
        .iter()
        .map(|tag| tag.translation_count.saturating_sub(1) as u64)
        .max()
        .unwrap_or(0);
    let uncategorized_tags = payload
        .categories
        .iter()
        .filter(|category| normalize_catalog_key(&category.name) == "other")
        .map(|category| -> Result<u64, PromptLexiconError> {
            let subcategories = slice_checked(
                &payload.subcategories,
                category.subcategory_start,
                category.subcategory_count,
                "category subcategory range",
            )?;
            Ok(subcategories
                .iter()
                .map(|subcategory| subcategory.tag_count as u64)
                .sum::<u64>())
        })
        .sum::<Result<u64, _>>()?;
    Ok(PromptLexiconStats {
        total_tags,
        categorized_tags: total_tags.saturating_sub(uncategorized_tags),
        uncategorized_tags,
        matched_weights,
        total_translations,
        tags_with_aliases,
        max_aliases_per_tag,
        source_count: payload.sources.len() as u64,
        manifest_version: 1,
        primary_from_category_json: 0,
        primary_from_manifest_sources: 0,
        primary_fallback_to_tag: 0,
    })
}
