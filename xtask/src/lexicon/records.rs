use super::{
    HashMap, Ordering, normalize_display_text, normalize_tag, normalize_tag_display, normalize_text,
};

#[derive(Clone)]
pub(super) struct TagRecord {
    pub(super) tag: String,
    pub(super) normalized_tag: String,
    pub(super) weight: Option<u64>,
    pub(super) category: Option<String>,
    pub(super) subcategory: Option<String>,
    pub(super) translations: HashMap<String, TranslationCandidate>,
    pub(super) next_translation_order: usize,
}

#[derive(Clone)]
pub(super) struct TranslationCandidate {
    text: String,
    normalized: String,
    order: usize,
    primary_priority: Option<i64>,
    primary_source_id: Option<String>,
}

#[derive(Clone)]
pub(super) struct TranslationInput {
    pub(super) priority: i64,
    pub(super) allow_primary: bool,
    pub(super) source_id: String,
}

impl TranslationInput {
    pub(super) const fn primary(priority: i64, source_id: String) -> Self {
        Self {
            priority,
            allow_primary: true,
            source_id,
        }
    }
}

#[derive(Clone)]
pub(super) struct ResolvedTranslations {
    pub(super) primary: String,
    pub(super) aliases: Vec<String>,
    pub(super) primary_source_id: Option<String>,
}

pub(super) fn ensure_tag_record<'a>(
    records: &'a mut HashMap<String, TagRecord>,
    raw_tag: &str,
) -> &'a mut TagRecord {
    let tag = normalize_tag_display(raw_tag);
    let normalized_tag = normalize_tag(&tag);
    records
        .entry(normalized_tag.clone())
        .or_insert_with(move || TagRecord {
            tag,
            normalized_tag,
            weight: None,
            category: None,
            subcategory: None,
            translations: HashMap::new(),
            next_translation_order: 0,
        })
}

pub(super) fn merge_translation_candidate(
    record: &mut TagRecord,
    raw_translation: &str,
    input: TranslationInput,
) {
    let translation = normalize_display_text(raw_translation);
    let normalized = normalize_text(&translation);
    if translation.is_empty()
        || normalized.is_empty()
        || normalized == "none"
        || normalized == record.normalized_tag
    {
        return;
    }
    let candidate = record
        .translations
        .entry(normalized.clone())
        .or_insert_with(|| {
            let order = record.next_translation_order;
            record.next_translation_order += 1;
            TranslationCandidate {
                text: translation,
                normalized,
                order,
                primary_priority: None,
                primary_source_id: None,
            }
        });
    if input.allow_primary
        && candidate
            .primary_priority
            .is_none_or(|priority| input.priority > priority)
    {
        candidate.primary_priority = Some(input.priority);
        candidate.primary_source_id = Some(input.source_id);
    }
}

pub(super) fn resolve_translations(record: &TagRecord) -> ResolvedTranslations {
    let primary_candidate = record
        .translations
        .values()
        .filter(|candidate| candidate.primary_priority.is_some())
        .max_by(|left, right| {
            left.primary_priority
                .cmp(&right.primary_priority)
                .then_with(|| right.order.cmp(&left.order))
        });
    let primary =
        primary_candidate.map_or_else(|| record.tag.clone(), |candidate| candidate.text.clone());
    let normalized_primary = normalize_text(&primary);
    let mut aliases = record
        .translations
        .values()
        .filter(|candidate| candidate.normalized != normalized_primary)
        .collect::<Vec<_>>();
    aliases.sort_by_key(|candidate| candidate.order);
    ResolvedTranslations {
        primary,
        aliases: aliases
            .into_iter()
            .map(|candidate| candidate.text.clone())
            .collect(),
        primary_source_id: primary_candidate
            .and_then(|candidate| candidate.primary_source_id.clone()),
    }
}

pub(super) fn compare_records_for_export(left: &TagRecord, right: &TagRecord) -> Ordering {
    right
        .weight
        .unwrap_or(0)
        .cmp(&left.weight.unwrap_or(0))
        .then_with(|| left.tag.to_lowercase().cmp(&right.tag.to_lowercase()))
}
