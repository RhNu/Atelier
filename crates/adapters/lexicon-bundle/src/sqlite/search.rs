use std::collections::HashSet;

use atelier_prompt_lexicon::{
    DanbooruCategory, LexiconContentRating, LexiconEntityKind, LexiconError, LexiconResult,
    LexiconSearchFilters, LexiconSearchItem, LexiconSearchQuery, canonical_comparison_key,
    normalized_search_text,
};
use rusqlite::{Connection, params, params_from_iter, types::Value as SqlValue};

use super::{map_search_item, row_u64, sql_error, sql_usize};

pub fn lexical_candidates(
    connection: &Connection,
    query: &str,
    candidate_limit: usize,
) -> LexiconResult<Vec<LexiconSearchItem>> {
    let normalized = normalized_search_text(query);
    if normalized.is_empty() {
        let mut statement = connection
            .prepare(
                "SELECT id, canonical_name, primary_translation, kind, category,
                        post_count, rating, canonical_name, 'browse', 0.0
                 FROM entities ORDER BY post_count DESC, canonical_name ASC LIMIT ?1",
            )
            .map_err(sql_error)?;
        return statement
            .query_map([sql_usize(candidate_limit)?], map_search_item)
            .map_err(sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_error);
    }

    let canonical = canonical_comparison_key(query);
    let prefix = format!("{canonical}*");
    let translation_prefix = format!("{normalized}*");
    let mut statement = connection
        .prepare(COMPLETION_SEARCH_SQL)
        .map_err(sql_error)?;
    statement
        .query_map(
            params![
                canonical,
                normalized,
                prefix,
                translation_prefix,
                sql_usize(candidate_limit)?
            ],
            map_search_item,
        )
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

pub fn lexical_search(
    connection: &Connection,
    query: &LexiconSearchQuery,
) -> LexiconResult<(Vec<LexiconSearchItem>, usize)> {
    let normalized = normalized_search_text(&query.text);
    let mut parameters = Vec::new();
    let sql = if normalized.is_empty() {
        let filters = filter_clause(&query.filters, &mut parameters);
        format!(
            "{BROWSE_SEARCH_SELECT}{filters}
             ORDER BY e.post_count DESC, e.canonical_name ASC LIMIT ? OFFSET ?"
        )
    } else {
        let canonical = canonical_comparison_key(&query.text);
        parameters.extend([
            SqlValue::Text(canonical.clone()),
            SqlValue::Text(normalized.clone()),
            SqlValue::Text(format!("{canonical}*")),
            SqlValue::Text(format!("{normalized}*")),
        ]);
        let filters = filter_clause(&query.filters, &mut parameters);
        format!(
            "{LEXICAL_SEARCH_PREFIX}{filters}
             GROUP BY e.id ORDER BY best.rank, e.post_count DESC, e.canonical_name ASC
             LIMIT ? OFFSET ?"
        )
    };
    parameters.push(SqlValue::Integer(sql_usize(query.limit)?));
    parameters.push(SqlValue::Integer(sql_usize(query.offset)?));
    collect_search_page(connection, &sql, parameters)
}

pub fn filter_items(
    connection: &Connection,
    items: Vec<LexiconSearchItem>,
    filters: &LexiconSearchFilters,
) -> LexiconResult<Vec<LexiconSearchItem>> {
    let group_members = if filters.group_ids.is_empty() {
        None
    } else {
        Some(group_member_ids(connection, &filters.group_ids)?)
    };
    Ok(items
        .into_iter()
        .filter(|item| filters.entity_kinds.is_empty() || filters.entity_kinds.contains(&item.kind))
        .filter(|item| filters.categories.is_empty() || filters.categories.contains(&item.category))
        .filter(|item| filters.ratings.is_empty() || filters.ratings.contains(&item.rating))
        .filter(|item| {
            group_members
                .as_ref()
                .is_none_or(|members| members.contains(&item.entity_id))
        })
        .collect())
}

fn collect_search_page(
    connection: &Connection,
    sql: &str,
    parameters: Vec<SqlValue>,
) -> LexiconResult<(Vec<LexiconSearchItem>, usize)> {
    let mut statement = connection.prepare(sql).map_err(sql_error)?;
    let rows = statement
        .query_map(params_from_iter(parameters), |row| {
            Ok((map_search_item(row)?, row_u64(row, 10)?))
        })
        .map_err(sql_error)?;
    let mut total = 0;
    let mut items = Vec::new();
    for row in rows {
        let (item, row_total) = row.map_err(sql_error)?;
        total = usize::try_from(row_total)
            .map_err(|_| LexiconError::query("lexicon result count exceeds platform size"))?;
        items.push(item);
    }
    Ok((items, total))
}

fn filter_clause(filters: &LexiconSearchFilters, parameters: &mut Vec<SqlValue>) -> String {
    let mut clauses = Vec::new();
    push_enum_filter(
        &mut clauses,
        parameters,
        "e.kind",
        filters.entity_kinds.iter().copied().map(entity_kind_value),
    );
    push_enum_filter(
        &mut clauses,
        parameters,
        "e.category",
        filters.categories.iter().copied().map(category_value),
    );
    push_enum_filter(
        &mut clauses,
        parameters,
        "e.rating",
        filters.ratings.iter().copied().map(rating_value),
    );
    for group_id in &filters.group_ids {
        clauses.push(
            "EXISTS (SELECT 1 FROM tag_group_members gm
                     WHERE gm.entity_id = e.id AND gm.group_id = ?)"
                .to_owned(),
        );
        parameters.push(SqlValue::Text(group_id.clone()));
    }
    if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    }
}

fn push_enum_filter<'a>(
    clauses: &mut Vec<String>,
    parameters: &mut Vec<SqlValue>,
    column: &str,
    values: impl Iterator<Item = &'a str>,
) {
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return;
    }
    clauses.push(format!(
        "{column} IN ({})",
        std::iter::repeat_n("?", values.len())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    parameters.extend(
        values
            .into_iter()
            .map(|value| SqlValue::Text(value.to_owned())),
    );
}

fn group_member_ids(connection: &Connection, group_ids: &[String]) -> LexiconResult<HashSet<u64>> {
    let mut members: Option<HashSet<u64>> = None;
    let mut statement = connection
        .prepare("SELECT entity_id FROM tag_group_members WHERE group_id = ?1")
        .map_err(sql_error)?;
    for group_id in group_ids {
        let current = statement
            .query_map([group_id], |row| row_u64(row, 0))
            .map_err(sql_error)?
            .collect::<Result<HashSet<u64>, _>>()
            .map_err(sql_error)?;
        members = Some(match members {
            Some(existing) => existing.intersection(&current).copied().collect(),
            None => current,
        });
    }
    Ok(members.unwrap_or_default())
}

const fn entity_kind_value(value: LexiconEntityKind) -> &'static str {
    match value {
        LexiconEntityKind::Tag => "tag",
        LexiconEntityKind::Artist => "artist",
    }
}

const fn category_value(value: DanbooruCategory) -> &'static str {
    match value {
        DanbooruCategory::General => "general",
        DanbooruCategory::Copyright => "copyright",
        DanbooruCategory::Character => "character",
        DanbooruCategory::Artist => "artist",
    }
}

const fn rating_value(value: LexiconContentRating) -> &'static str {
    match value {
        LexiconContentRating::Safe => "safe",
        LexiconContentRating::Sensitive => "sensitive",
        LexiconContentRating::Unknown => "unknown",
    }
}

const COMPLETION_SEARCH_SQL: &str = "
WITH matches AS (
    SELECT id AS entity_id, 0 AS rank, canonical_name AS matched_text, 'canonical_exact' AS reason
      FROM entities WHERE normalized_name = ?1
    UNION ALL
    SELECT entity_id, 1, alias, 'alias_exact' FROM aliases WHERE normalized_alias = ?1
    UNION ALL
    SELECT entity_id, 2, text, 'translation_exact' FROM translations WHERE normalized_text = ?2
    UNION ALL
    SELECT id, 3, canonical_name, 'canonical_prefix' FROM entities WHERE normalized_name GLOB ?3
    UNION ALL
    SELECT entity_id, 4, alias, 'alias_prefix' FROM aliases WHERE normalized_alias GLOB ?3
    UNION ALL
    SELECT entity_id, 5, text, 'translation_prefix' FROM translations WHERE normalized_text GLOB ?4
), best AS (
    SELECT entity_id, MIN(rank) AS rank FROM matches GROUP BY entity_id
)
SELECT e.id, e.canonical_name, e.primary_translation, e.kind, e.category,
       e.post_count, e.rating, m.matched_text, m.reason,
       CAST(100 - best.rank AS REAL)
FROM best JOIN matches m ON m.entity_id = best.entity_id AND m.rank = best.rank
JOIN entities e ON e.id = best.entity_id
GROUP BY e.id ORDER BY best.rank, e.post_count DESC, e.canonical_name ASC LIMIT ?5";

const BROWSE_SEARCH_SELECT: &str = "
SELECT e.id, e.canonical_name, e.primary_translation, e.kind, e.category,
       e.post_count, e.rating, e.canonical_name, 'browse', 0.0,
       COUNT(*) OVER()
FROM entities e";

const LEXICAL_SEARCH_PREFIX: &str = "
WITH matches AS (
    SELECT id AS entity_id, 0 AS rank, canonical_name AS matched_text, 'canonical_exact' AS reason
      FROM entities WHERE normalized_name = ?1
    UNION ALL SELECT entity_id, 1, alias, 'alias_exact' FROM aliases WHERE normalized_alias = ?1
    UNION ALL SELECT entity_id, 2, text, 'translation_exact' FROM translations WHERE normalized_text = ?2
    UNION ALL SELECT id, 3, canonical_name, 'canonical_prefix' FROM entities WHERE normalized_name GLOB ?3
    UNION ALL SELECT entity_id, 4, alias, 'alias_prefix' FROM aliases WHERE normalized_alias GLOB ?3
    UNION ALL SELECT entity_id, 5, text, 'translation_prefix' FROM translations WHERE normalized_text GLOB ?4
    UNION ALL
    SELECT entity_id, 6, canonical_name, 'full_text'
      FROM entity_fts WHERE entity_fts MATCH ('\"' || replace(?2, '\"', ' ') || '\" OR ' || replace(?2, '\"', ' ') || '*')
), best AS (
    SELECT entity_id, MIN(rank) AS rank FROM matches GROUP BY entity_id
)
SELECT e.id, e.canonical_name, e.primary_translation, e.kind, e.category,
       e.post_count, e.rating, m.matched_text, m.reason,
       CAST(100 - best.rank AS REAL), COUNT(*) OVER()
FROM best JOIN matches m ON m.entity_id = best.entity_id AND m.rank = best.rank
JOIN entities e ON e.id = best.entity_id";

#[cfg(test)]
mod tests {
    use atelier_prompt_lexicon::{LexiconSearchFilters, LexiconSearchMode, LexiconSearchQuery};

    use super::lexical_search;

    #[test]
    fn browse_filters_before_pagination_and_reports_the_full_total() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE entities(
                    id INTEGER PRIMARY KEY,
                    canonical_name TEXT NOT NULL,
                    primary_translation TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    category TEXT NOT NULL,
                    post_count INTEGER NOT NULL,
                    rating TEXT NOT NULL
                );
                CREATE TABLE tag_group_members(group_id TEXT NOT NULL, entity_id INTEGER NOT NULL);
                INSERT INTO entities VALUES
                    (1, 'global_popular', '', 'tag', 'general', 1000, 'safe'),
                    (2, 'group_first', '', 'tag', 'general', 100, 'safe'),
                    (3, 'group_second', '', 'tag', 'general', 10, 'safe');
                INSERT INTO tag_group_members VALUES ('verbs', 2), ('verbs', 3);",
            )
            .unwrap();
        let query = LexiconSearchQuery {
            text: String::new(),
            mode: LexiconSearchMode::Lexical,
            filters: LexiconSearchFilters {
                group_ids: vec!["verbs".to_owned()],
                ..LexiconSearchFilters::default()
            },
            selected_entity_ids: Vec::new(),
            offset: 0,
            limit: 1,
        };

        let (items, total) = lexical_search(&connection, &query).unwrap();

        assert_eq!(total, 2);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].canonical_name, "group_first");
    }

    #[test]
    fn text_search_filters_matches_before_pagination() {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE entities(
                    id INTEGER PRIMARY KEY,
                    canonical_name TEXT NOT NULL,
                    normalized_name TEXT NOT NULL,
                    primary_translation TEXT NOT NULL,
                    kind TEXT NOT NULL,
                    category TEXT NOT NULL,
                    post_count INTEGER NOT NULL,
                    rating TEXT NOT NULL
                );
                CREATE TABLE aliases(
                    entity_id INTEGER NOT NULL,
                    alias TEXT NOT NULL,
                    normalized_alias TEXT NOT NULL
                );
                CREATE TABLE translations(
                    entity_id INTEGER NOT NULL,
                    text TEXT NOT NULL,
                    normalized_text TEXT NOT NULL
                );
                CREATE TABLE tag_group_members(group_id TEXT NOT NULL, entity_id INTEGER NOT NULL);
                CREATE VIRTUAL TABLE entity_fts USING fts5(
                    entity_id UNINDEXED,
                    canonical_name
                );
                INSERT INTO entities VALUES
                    (1, 'pose', 'pose', '', 'tag', 'general', 1000, 'safe'),
                    (2, 'pose_a', 'pose_a', '', 'tag', 'general', 100, 'safe'),
                    (3, 'pose_b', 'pose_b', '', 'tag', 'general', 10, 'safe');
                INSERT INTO tag_group_members VALUES ('poses', 2), ('poses', 3);
                INSERT INTO entity_fts VALUES (1, 'pose'), (2, 'pose_a'), (3, 'pose_b');",
            )
            .unwrap();
        let query = LexiconSearchQuery {
            text: "pose".to_owned(),
            mode: LexiconSearchMode::Lexical,
            filters: LexiconSearchFilters {
                group_ids: vec!["poses".to_owned()],
                ..LexiconSearchFilters::default()
            },
            selected_entity_ids: Vec::new(),
            offset: 0,
            limit: 1,
        };

        let (items, total) = lexical_search(&connection, &query).unwrap();

        assert_eq!(total, 2);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].canonical_name, "pose_a");
    }
}
