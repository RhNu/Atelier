use std::collections::{HashMap, HashSet};
use std::path::Path;

use atelier_prompt_lexicon::{
    DanbooruCategory, LexiconContentRating, LexiconEntityDetail, LexiconEntityKind, LexiconError,
    LexiconFacet, LexiconGroupSummary, LexiconMatchReason, LexiconRelatedEntity, LexiconResult,
    LexiconSearchItem, LocalizedLexiconText, ResolvedLexiconEntity,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};

mod search;

pub use search::{filter_items, lexical_candidates, lexical_search};

pub fn open_read_only(path: &Path) -> LexiconResult<Connection> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sql_error)?;
    connection
        .pragma_update(None, "query_only", true)
        .map_err(sql_error)?;
    connection
        .pragma_update(None, "trusted_schema", false)
        .map_err(sql_error)?;
    Ok(connection)
}

pub fn validate_database(connection: &Connection) -> LexiconResult<()> {
    let format: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'format'",
            [],
            |row| row.get(0),
        )
        .map_err(sql_error)?;
    let schema: String = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .map_err(sql_error)?;
    if format != super::manifest::BUNDLE_FORMAT
        || schema != super::manifest::BUNDLE_SCHEMA_VERSION.to_string()
    {
        return Err(LexiconError::invalid_bundle(format!(
            "SQLite format {format} schema {schema} is unsupported"
        )));
    }
    let quick_check: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(sql_error)?;
    if quick_check != "ok" {
        return Err(LexiconError::invalid_bundle(format!(
            "SQLite quick_check failed: {quick_check}"
        )));
    }
    Ok(())
}

pub fn stats(connection: &Connection) -> LexiconResult<atelier_prompt_lexicon::LexiconStats> {
    connection
        .query_row(
            "SELECT
                COUNT(*),
                SUM(CASE WHEN kind = 'tag' THEN 1 ELSE 0 END),
                SUM(CASE WHEN kind = 'artist' THEN 1 ELSE 0 END),
                SUM(CASE WHEN rating = 'sensitive' THEN 1 ELSE 0 END),
                (SELECT COUNT(*) FROM translations),
                (SELECT COUNT(*) FROM tag_groups)
             FROM entities",
            [],
            |row| {
                Ok(atelier_prompt_lexicon::LexiconStats {
                    total_entities: row_u64(row, 0)?,
                    tag_entities: row_u64(row, 1)?,
                    artist_entities: row_u64(row, 2)?,
                    sensitive_entities: row_u64(row, 3)?,
                    translation_count: row_u64(row, 4)?,
                    group_count: row_u64(row, 5)?,
                })
            },
        )
        .map_err(sql_error)
}

pub fn category_facets(connection: &Connection) -> LexiconResult<Vec<LexiconFacet>> {
    let mut statement = connection
        .prepare(
            "SELECT category, COUNT(*) FROM entities GROUP BY category
             ORDER BY CASE category
                WHEN 'general' THEN 0 WHEN 'copyright' THEN 1
                WHEN 'character' THEN 2 ELSE 3 END",
        )
        .map_err(sql_error)?;
    statement
        .query_map([], |row| {
            let value: String = row.get(0)?;
            Ok(LexiconFacet {
                label: value.clone(),
                value,
                count: row_u64(row, 1)?,
            })
        })
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

pub fn groups(connection: &Connection) -> LexiconResult<Vec<LexiconGroupSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT id, name, member_count FROM tag_groups
             ORDER BY member_count DESC, name ASC",
        )
        .map_err(sql_error)?;
    statement
        .query_map([], map_group)
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

pub fn details(connection: &Connection, entity_id: u64) -> LexiconResult<LexiconEntityDetail> {
    let entity = entity_by_id(connection, entity_id)?.ok_or(LexiconError::NotFound(entity_id))?;
    let translations = localized_texts(
        connection,
        "SELECT locale, text FROM translations WHERE entity_id = ?1 ORDER BY locale",
        entity_id,
    )?;
    let wiki = localized_texts(
        connection,
        "SELECT locale, text FROM wiki WHERE entity_id = ?1 ORDER BY locale",
        entity_id,
    )?;
    let aliases = strings(
        connection,
        "SELECT alias FROM aliases WHERE entity_id = ?1 ORDER BY alias",
        entity_id,
    )?;
    let groups = entity_groups(connection, entity_id)?;
    let related = related_entities(connection, entity_id)?;
    Ok(LexiconEntityDetail {
        entity,
        translations,
        aliases,
        wiki,
        groups,
        related,
    })
}

pub fn resolve_entities(
    connection: &Connection,
    entity_ids: &[u64],
) -> LexiconResult<Vec<ResolvedLexiconEntity>> {
    if entity_ids.is_empty() {
        return Err(LexiconError::invalid_request(
            "at least one entity_id is required",
        ));
    }
    if entity_ids.len() > 500 {
        return Err(LexiconError::invalid_request(
            "no more than 500 entities can be appended at once",
        ));
    }
    let mut seen = HashSet::new();
    let mut resolved = Vec::with_capacity(entity_ids.len());
    for entity_id in entity_ids {
        if !seen.insert(*entity_id) {
            continue;
        }
        let sql_entity_id = sql_u64(*entity_id)?;
        let canonical_name: Option<String> = connection
            .query_row(
                "SELECT canonical_name FROM entities WHERE id = ?1",
                [sql_entity_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error)?;
        let canonical_name = canonical_name.ok_or(LexiconError::NotFound(*entity_id))?;
        let aliases = strings(
            connection,
            "SELECT alias FROM aliases WHERE entity_id = ?1 ORDER BY alias",
            *entity_id,
        )?;
        resolved.push(ResolvedLexiconEntity {
            entity_id: *entity_id,
            canonical_name,
            aliases,
        });
    }
    Ok(resolved)
}

pub fn context_scores(
    connection: &Connection,
    selected: &[u64],
) -> LexiconResult<HashMap<u64, f32>> {
    let mut scores = HashMap::<u64, f32>::new();
    let mut statement = connection
        .prepare(
            "SELECT target_entity_id, npmi FROM related_entities
             WHERE source_entity_id = ?1 AND npmi > 0",
        )
        .map_err(sql_error)?;
    for entity_id in selected.iter().take(100) {
        let sql_entity_id = sql_u64(*entity_id)?;
        let rows = statement
            .query_map([sql_entity_id], |row| {
                Ok((row_u64(row, 0)?, row.get::<_, f32>(1)?))
            })
            .map_err(sql_error)?;
        for row in rows {
            let (target, score) = row.map_err(sql_error)?;
            scores
                .entry(target)
                .and_modify(|current| *current = current.max(score))
                .or_insert(score);
        }
    }
    Ok(scores)
}

pub fn all_semantic_rows(connection: &Connection) -> LexiconResult<Vec<LexiconSearchItem>> {
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.canonical_name, e.primary_translation, e.kind, e.category,
                    e.post_count, e.rating, e.canonical_name, 'semantic', 0.0
             FROM semantic_rows s JOIN entities e ON e.id = s.entity_id
             ORDER BY s.row_index",
        )
        .map_err(sql_error)?;
    statement
        .query_map([], map_search_item)
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

fn entity_by_id(
    connection: &Connection,
    entity_id: u64,
) -> LexiconResult<Option<LexiconSearchItem>> {
    let entity_id = sql_u64(entity_id)?;
    connection
        .query_row(
            "SELECT id, canonical_name, primary_translation, kind, category,
                    post_count, rating, canonical_name, 'browse', 0.0
             FROM entities WHERE id = ?1",
            [entity_id],
            map_search_item,
        )
        .optional()
        .map_err(sql_error)
}

fn localized_texts(
    connection: &Connection,
    sql: &str,
    entity_id: u64,
) -> LexiconResult<Vec<LocalizedLexiconText>> {
    let entity_id = sql_u64(entity_id)?;
    let mut statement = connection.prepare(sql).map_err(sql_error)?;
    statement
        .query_map([entity_id], |row| {
            Ok(LocalizedLexiconText {
                locale: row.get(0)?,
                text: row.get(1)?,
            })
        })
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

fn strings(connection: &Connection, sql: &str, entity_id: u64) -> LexiconResult<Vec<String>> {
    let entity_id = sql_u64(entity_id)?;
    let mut statement = connection.prepare(sql).map_err(sql_error)?;
    statement
        .query_map([entity_id], |row| row.get(0))
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

fn entity_groups(
    connection: &Connection,
    entity_id: u64,
) -> LexiconResult<Vec<LexiconGroupSummary>> {
    let entity_id = sql_u64(entity_id)?;
    let mut statement = connection
        .prepare(
            "SELECT g.id, g.name, g.member_count FROM tag_groups g
             JOIN tag_group_members m ON m.group_id = g.id
             WHERE m.entity_id = ?1 ORDER BY g.name",
        )
        .map_err(sql_error)?;
    statement
        .query_map([entity_id], map_group)
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

fn related_entities(
    connection: &Connection,
    entity_id: u64,
) -> LexiconResult<Vec<LexiconRelatedEntity>> {
    let entity_id = sql_u64(entity_id)?;
    let mut statement = connection
        .prepare(
            "SELECT e.id, e.canonical_name, e.primary_translation, e.kind, e.category,
                    e.post_count, e.rating, e.canonical_name, 'browse', r.npmi,
                    r.relation
             FROM related_entities r JOIN entities e ON e.id = r.target_entity_id
             WHERE r.source_entity_id = ?1 AND r.npmi > 0
             ORDER BY r.npmi DESC, e.post_count DESC LIMIT 30",
        )
        .map_err(sql_error)?;
    statement
        .query_map([entity_id], |row| {
            Ok(LexiconRelatedEntity {
                entity: map_search_item(row)?,
                relation: row.get(10)?,
                score: row.get(9)?,
            })
        })
        .map_err(sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error)
}

fn map_group(row: &rusqlite::Row<'_>) -> rusqlite::Result<LexiconGroupSummary> {
    Ok(LexiconGroupSummary {
        id: row.get(0)?,
        name: row.get(1)?,
        member_count: row_u64(row, 2)?,
    })
}

fn map_search_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<LexiconSearchItem> {
    let kind: String = row.get(3)?;
    let category: String = row.get(4)?;
    let rating: String = row.get(6)?;
    let reason: String = row.get(8)?;
    Ok(LexiconSearchItem {
        entity_id: row_u64(row, 0)?,
        canonical_name: row.get(1)?,
        primary_translation: row.get(2)?,
        kind: parse_kind(&kind)?,
        category: parse_category(&category)?,
        post_count: row_u64(row, 5)?,
        rating: parse_rating(&rating)?,
        matched_text: row.get(7)?,
        match_reason: parse_reason(&reason)?,
        score: row.get(9)?,
    })
}

fn parse_kind(value: &str) -> rusqlite::Result<LexiconEntityKind> {
    match value {
        "tag" => Ok(LexiconEntityKind::Tag),
        "artist" => Ok(LexiconEntityKind::Artist),
        other => Err(conversion_error(format!("invalid entity kind {other}"))),
    }
}

fn parse_category(value: &str) -> rusqlite::Result<DanbooruCategory> {
    match value {
        "general" => Ok(DanbooruCategory::General),
        "copyright" => Ok(DanbooruCategory::Copyright),
        "character" => Ok(DanbooruCategory::Character),
        "artist" => Ok(DanbooruCategory::Artist),
        other => Err(conversion_error(format!("invalid category {other}"))),
    }
}

fn parse_rating(value: &str) -> rusqlite::Result<LexiconContentRating> {
    match value {
        "safe" => Ok(LexiconContentRating::Safe),
        "sensitive" => Ok(LexiconContentRating::Sensitive),
        "unknown" => Ok(LexiconContentRating::Unknown),
        other => Err(conversion_error(format!("invalid rating {other}"))),
    }
}

fn parse_reason(value: &str) -> rusqlite::Result<LexiconMatchReason> {
    match value {
        "canonical_exact" => Ok(LexiconMatchReason::CanonicalExact),
        "alias_exact" => Ok(LexiconMatchReason::AliasExact),
        "translation_exact" => Ok(LexiconMatchReason::TranslationExact),
        "canonical_prefix" => Ok(LexiconMatchReason::CanonicalPrefix),
        "alias_prefix" => Ok(LexiconMatchReason::AliasPrefix),
        "translation_prefix" => Ok(LexiconMatchReason::TranslationPrefix),
        "full_text" => Ok(LexiconMatchReason::FullText),
        "semantic" => Ok(LexiconMatchReason::Semantic),
        "browse" => Ok(LexiconMatchReason::Browse),
        other => Err(conversion_error(format!("invalid match reason {other}"))),
    }
}

fn conversion_error(message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        std::io::Error::new(std::io::ErrorKind::InvalidData, message).into(),
    )
}

fn row_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value)
        .map_err(|_| conversion_error(format!("negative integer at column {index}")))
}

fn sql_u64(value: u64) -> LexiconResult<i64> {
    i64::try_from(value)
        .map_err(|_| LexiconError::invalid_request("entity id exceeds SQLite integer range"))
}

fn sql_usize(value: usize) -> LexiconResult<i64> {
    i64::try_from(value)
        .map_err(|_| LexiconError::invalid_request("query limit exceeds SQLite integer range"))
}

#[allow(clippy::needless_pass_by_value)]
fn sql_error(error: rusqlite::Error) -> LexiconError {
    LexiconError::query(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::open_read_only;

    #[test]
    fn bundle_connections_reject_writes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("lexicon.sqlite");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE fixture(id INTEGER);")
            .unwrap();
        drop(connection);

        let connection = open_read_only(&path).unwrap();
        assert!(
            connection
                .execute("INSERT INTO fixture(id) VALUES (1)", [])
                .is_err()
        );
    }
}
