pub const CREATE_SCHEMA: &str = r"
PRAGMA page_size = 4096;
PRAGMA journal_mode = OFF;
PRAGMA synchronous = OFF;
PRAGMA temp_store = MEMORY;

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE entities (
    id INTEGER PRIMARY KEY,
    canonical_name TEXT NOT NULL UNIQUE,
    normalized_name TEXT NOT NULL,
    primary_translation TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('tag', 'artist')),
    category TEXT NOT NULL CHECK (category IN ('general', 'copyright', 'character', 'artist')),
    post_count INTEGER NOT NULL CHECK (post_count >= 0),
    rating TEXT NOT NULL CHECK (rating IN ('safe', 'sensitive', 'unknown'))
);
CREATE INDEX entities_normalized_name ON entities(normalized_name);
CREATE INDEX entities_category_post_count ON entities(category, post_count DESC);
CREATE INDEX entities_kind_post_count ON entities(kind, post_count DESC);
CREATE INDEX entities_rating ON entities(rating);

CREATE TABLE aliases (
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    alias TEXT NOT NULL,
    normalized_alias TEXT NOT NULL,
    PRIMARY KEY (entity_id, alias)
) WITHOUT ROWID;
CREATE INDEX aliases_normalized ON aliases(normalized_alias, entity_id);

CREATE TABLE translations (
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    locale TEXT NOT NULL,
    text TEXT NOT NULL,
    normalized_text TEXT NOT NULL,
    PRIMARY KEY (entity_id, locale, text)
) WITHOUT ROWID;
CREATE INDEX translations_normalized ON translations(normalized_text, entity_id);

CREATE TABLE wiki (
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    locale TEXT NOT NULL,
    text TEXT NOT NULL,
    PRIMARY KEY (entity_id, locale)
) WITHOUT ROWID;

CREATE TABLE tag_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    member_count INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TABLE tag_group_members (
    group_id TEXT NOT NULL REFERENCES tag_groups(id),
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    PRIMARY KEY (group_id, entity_id)
) WITHOUT ROWID;
CREATE INDEX tag_group_members_entity ON tag_group_members(entity_id, group_id);

CREATE TABLE related_entities (
    source_entity_id INTEGER NOT NULL REFERENCES entities(id),
    target_entity_id INTEGER NOT NULL REFERENCES entities(id),
    relation TEXT NOT NULL,
    npmi REAL NOT NULL,
    PRIMARY KEY (source_entity_id, target_entity_id, relation)
) WITHOUT ROWID;
CREATE INDEX related_entities_source_score
    ON related_entities(source_entity_id, npmi DESC);

CREATE TABLE semantic_rows (
    row_index INTEGER PRIMARY KEY,
    entity_id INTEGER NOT NULL UNIQUE REFERENCES entities(id)
);

CREATE VIRTUAL TABLE entity_fts USING fts5(
    entity_id UNINDEXED,
    canonical_name,
    aliases,
    translations,
    wiki,
    groups_text,
    tokenize = 'unicode61 remove_diacritics 2'
);
";
