use super::{
    HashMap, ManifestSource, SourceParser, TagRecord, TranslationInput, ensure_tag_record, fs,
    is_valid_tag, merge_translation_candidate, normalize_display_text, normalize_tag,
    normalize_tag_display, strip_bom,
};

pub(super) fn ingest_source(
    source: &ManifestSource,
    records: &mut HashMap<String, TagRecord>,
) -> Result<(), String> {
    let raw_content = fs::read_to_string(&source.path).map_err(|error| error.to_string())?;
    let content = strip_bom(&raw_content);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(content.as_bytes());
    for row in reader.records() {
        let columns = row.map_err(|error| error.to_string())?;
        let Some(row) = parse_source_row(source, &columns) else {
            continue;
        };
        if !is_valid_tag(&row.tag) {
            continue;
        }
        let record = ensure_tag_record(records, &row.tag);
        if source.parser == SourceParser::Weighted {
            record.tag = normalize_tag_display(&row.tag);
        }
        if let Some(weight) = row.weight
            && record.weight.is_none_or(|existing| weight > existing)
        {
            record.weight = Some(weight);
        }
        for translation in row.translations {
            merge_translation_candidate(
                record,
                &translation.text,
                TranslationInput {
                    priority: source.priority,
                    allow_primary: translation.allow_primary
                        && source.allow_primary
                        && !source.alias_only,
                    source_id: source.id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn parse_source_row(source: &ManifestSource, columns: &csv::StringRecord) -> Option<SourceRow> {
    match source.parser {
        SourceParser::Weighted => parse_weighted_csv_row(columns),
        SourceParser::Simple => parse_simple_csv_row(source, columns),
        SourceParser::Reversed => parse_reversed_csv_row(source, columns),
        SourceParser::Github => parse_github_csv_row(source, columns),
        SourceParser::Alias => parse_alias_csv_row(columns),
    }
}

fn parse_weighted_csv_row(columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() < 3 {
        return None;
    }
    let tag = columns.get(0)?;
    if normalize_tag(tag) == "tag" {
        return None;
    }
    let weight = columns.get(1)?.parse::<u64>().unwrap_or(0);
    let translation = columns.get(2)?;
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: Some(weight),
        translations: translation_items(translation, true),
    })
}

fn parse_simple_csv_row(source: &ManifestSource, columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() < 2 {
        return None;
    }
    let tag = columns.get(0)?;
    if normalize_tag(tag) == "tag" {
        return None;
    }
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: translation_items(columns.get(1)?, source.allow_primary),
    })
}

fn parse_reversed_csv_row(
    source: &ManifestSource,
    columns: &csv::StringRecord,
) -> Option<SourceRow> {
    if columns.len() < 2 {
        return None;
    }
    let tag = columns.get(1)?;
    if normalize_tag(tag) == "tag" {
        return None;
    }
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: translation_items(columns.get(0).unwrap_or_default(), source.allow_primary),
    })
}

fn parse_github_csv_row(source: &ManifestSource, columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() == 2 {
        if normalize_tag(columns.get(0)?) == "tag" {
            return None;
        }
        return Some(SourceRow {
            tag: columns.get(0)?.to_owned(),
            weight: None,
            translations: split_github_translations(columns.get(1)?, source),
        });
    }
    if columns.len() < 4 || columns.get(0)?.trim() == "danbooru_text" {
        return None;
    }
    let tag = columns.get(2)?;
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: split_github_translations(columns.get(3)?, source),
    })
}

fn parse_alias_csv_row(columns: &csv::StringRecord) -> Option<SourceRow> {
    if columns.len() < 4 {
        return None;
    }
    let tag = columns.get(0)?;
    if tag.trim() == "tag" {
        return None;
    }
    Some(SourceRow {
        tag: tag.to_owned(),
        weight: None,
        translations: split_translation_list(columns.get(3)?)
            .into_iter()
            .map(|text| TranslationRow {
                text,
                allow_primary: false,
            })
            .collect(),
    })
}

fn split_github_translations(
    raw_translations: &str,
    source: &ManifestSource,
) -> Vec<TranslationRow> {
    split_translation_list(raw_translations)
        .into_iter()
        .enumerate()
        .map(|(index, text)| TranslationRow {
            text,
            allow_primary: index == 0 && source.allow_primary,
        })
        .collect()
}

#[derive(Clone)]
struct SourceRow {
    tag: String,
    weight: Option<u64>,
    translations: Vec<TranslationRow>,
}

#[derive(Clone)]
struct TranslationRow {
    text: String,
    allow_primary: bool,
}

fn translation_items(value: &str, allow_primary: bool) -> Vec<TranslationRow> {
    let text = normalize_display_text(value);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![TranslationRow {
            text,
            allow_primary,
        }]
    }
}

fn split_translation_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(normalize_display_text)
        .filter(|value| !value.is_empty())
        .collect()
}
