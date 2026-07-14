use super::{
    Ordering, PromptLexiconEntry, PromptLexiconMatchField, PromptLexiconMatchRank,
    normalize_search_text,
};

#[derive(Clone, Debug)]
pub struct SearchEntry {
    tag: String,
    weight: Option<u64>,
    category: String,
    subcategory: String,
    primary_translation: String,
    aliases: Vec<String>,
    normalized_tag: String,
    normalized_primary_translation: String,
    normalized_aliases: Vec<String>,
}

impl SearchEntry {
    pub(super) fn new(
        tag: String,
        weight: Option<u64>,
        category: String,
        subcategory: String,
        primary_translation: String,
        aliases: Vec<String>,
    ) -> Self {
        Self {
            normalized_tag: normalize_search_text(&tag),
            normalized_primary_translation: normalize_search_text(&primary_translation),
            normalized_aliases: aliases
                .iter()
                .map(|item| normalize_search_text(item))
                .collect(),
            tag,
            weight,
            category,
            subcategory,
            primary_translation,
            aliases,
        }
    }

    pub(super) fn match_query(&self, query: &str) -> Option<SearchMatch<'_>> {
        if query.is_empty() {
            return None;
        }
        for rank in [
            PromptLexiconMatchRank::Exact,
            PromptLexiconMatchRank::Prefix,
            PromptLexiconMatchRank::Substring,
        ] {
            if matches_query(&self.normalized_tag, query, rank) {
                return Some(SearchMatch::new(
                    self,
                    PromptLexiconMatchField::Tag,
                    rank,
                    self.primary_translation.as_str(),
                ));
            }
            if matches_query(&self.normalized_primary_translation, query, rank) {
                return Some(SearchMatch::new(
                    self,
                    PromptLexiconMatchField::PrimaryTranslation,
                    rank,
                    self.primary_translation.as_str(),
                ));
            }
            if let Some(alias) =
                find_alias_match(query, &self.aliases, &self.normalized_aliases, rank)
            {
                return Some(SearchMatch::new(
                    self,
                    PromptLexiconMatchField::Alias,
                    rank,
                    alias,
                ));
            }
        }
        None
    }

    pub(super) fn to_browse_entry(&self) -> PromptLexiconEntry {
        PromptLexiconEntry {
            tag: self.tag.clone(),
            weight: self.weight,
            category: self.category.clone(),
            subcategory: self.subcategory.clone(),
            primary_translation: self.primary_translation.clone(),
            matched_translation: self.primary_translation.clone(),
            match_field: PromptLexiconMatchField::Tag,
            match_rank: PromptLexiconMatchRank::Substring,
        }
    }

    pub(super) fn normalized_values(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.normalized_tag.as_str())
            .chain(std::iter::once(
                self.normalized_primary_translation.as_str(),
            ))
            .chain(self.normalized_aliases.iter().map(String::as_str))
    }
}

pub struct SearchMatch<'a> {
    entry: &'a SearchEntry,
    match_field: PromptLexiconMatchField,
    match_rank: PromptLexiconMatchRank,
    matched_translation: &'a str,
}

impl<'a> SearchMatch<'a> {
    pub(super) const fn new(
        entry: &'a SearchEntry,
        match_field: PromptLexiconMatchField,
        match_rank: PromptLexiconMatchRank,
        matched_translation: &'a str,
    ) -> Self {
        Self {
            entry,
            match_field,
            match_rank,
            matched_translation,
        }
    }

    pub(super) fn into_entry(self) -> PromptLexiconEntry {
        PromptLexiconEntry {
            tag: self.entry.tag.clone(),
            weight: self.entry.weight,
            category: self.entry.category.clone(),
            subcategory: self.entry.subcategory.clone(),
            primary_translation: self.entry.primary_translation.clone(),
            matched_translation: self.matched_translation.to_owned(),
            match_field: self.match_field,
            match_rank: self.match_rank,
        }
    }
}

pub fn compare_search_match(left: &SearchMatch<'_>, right: &SearchMatch<'_>) -> Ordering {
    rank_priority(left.match_rank)
        .cmp(&rank_priority(right.match_rank))
        .then_with(|| field_priority(left.match_field).cmp(&field_priority(right.match_field)))
        .then_with(|| {
            right
                .entry
                .weight
                .unwrap_or(0)
                .cmp(&left.entry.weight.unwrap_or(0))
        })
        .then_with(|| left.entry.tag.cmp(&right.entry.tag))
}

pub fn insert_sorted_match<'a>(
    matches: &mut Vec<SearchMatch<'a>>,
    candidate: SearchMatch<'a>,
    limit: usize,
) {
    if limit == 0 {
        return;
    }
    let position = matches
        .binary_search_by(|probe| compare_search_match(probe, &candidate))
        .unwrap_or_else(|position| position);
    if position >= limit {
        return;
    }
    matches.insert(position, candidate);
    if matches.len() > limit {
        matches.pop();
    }
}

pub fn compare_browse_entries(left: &SearchEntry, right: &SearchEntry) -> Ordering {
    right
        .weight
        .unwrap_or(0)
        .cmp(&left.weight.unwrap_or(0))
        .then_with(|| left.tag.cmp(&right.tag))
}

const fn rank_priority(rank: PromptLexiconMatchRank) -> u8 {
    match rank {
        PromptLexiconMatchRank::Exact => 0,
        PromptLexiconMatchRank::Prefix => 1,
        PromptLexiconMatchRank::Substring => 2,
    }
}

const fn field_priority(field: PromptLexiconMatchField) -> u8 {
    match field {
        PromptLexiconMatchField::Tag => 0,
        PromptLexiconMatchField::PrimaryTranslation => 1,
        PromptLexiconMatchField::Alias => 2,
    }
}

fn matches_query(value: &str, query: &str, rank: PromptLexiconMatchRank) -> bool {
    match rank {
        PromptLexiconMatchRank::Exact => value == query,
        PromptLexiconMatchRank::Prefix => value.starts_with(query),
        PromptLexiconMatchRank::Substring => value.contains(query),
    }
}

fn find_alias_match<'a>(
    query: &str,
    aliases: &'a [String],
    normalized_aliases: &[String],
    rank: PromptLexiconMatchRank,
) -> Option<&'a str> {
    aliases
        .iter()
        .zip(normalized_aliases)
        .find_map(|(alias, normalized)| {
            matches_query(normalized, query, rank).then_some(alias.as_str())
        })
}
