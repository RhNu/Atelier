#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LexiconEntityKind {
    Tag,
    Artist,
}

impl LexiconEntityKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tag => "tag",
            Self::Artist => "artist",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DanbooruCategory {
    General,
    Copyright,
    Character,
    Artist,
}

impl DanbooruCategory {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Copyright => "copyright",
            Self::Character => "character",
            Self::Artist => "artist",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LexiconContentRating {
    Safe,
    Sensitive,
    Unknown,
}

impl LexiconContentRating {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Sensitive => "sensitive",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LexiconSearchMode {
    Lexical,
    Semantic,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LexiconMatchReason {
    CanonicalExact,
    AliasExact,
    TranslationExact,
    CanonicalPrefix,
    AliasPrefix,
    TranslationPrefix,
    FullText,
    Semantic,
    Browse,
}

impl LexiconMatchReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalExact => "canonical_exact",
            Self::AliasExact => "alias_exact",
            Self::TranslationExact => "translation_exact",
            Self::CanonicalPrefix => "canonical_prefix",
            Self::AliasPrefix => "alias_prefix",
            Self::TranslationPrefix => "translation_prefix",
            Self::FullText => "full_text",
            Self::Semantic => "semantic",
            Self::Browse => "browse",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconCapabilityStatus {
    pub lexical_available: bool,
    pub semantic_available: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LexiconStats {
    pub total_entities: u64,
    pub tag_entities: u64,
    pub artist_entities: u64,
    pub sensitive_entities: u64,
    pub translation_count: u64,
    pub group_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconFacet {
    pub value: String,
    pub label: String,
    pub count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconGroupSummary {
    pub id: String,
    pub name: String,
    pub member_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconBootstrap {
    pub bundle_version: Option<String>,
    pub status: LexiconCapabilityStatus,
    pub stats: LexiconStats,
    pub categories: Vec<LexiconFacet>,
    pub groups: Vec<LexiconGroupSummary>,
}

impl LexiconBootstrap {
    #[must_use]
    pub fn unavailable(message: String) -> Self {
        Self {
            bundle_version: None,
            status: LexiconCapabilityStatus {
                lexical_available: false,
                semantic_available: false,
                message: Some(message),
            },
            stats: LexiconStats::default(),
            categories: Vec::new(),
            groups: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexiconSearchItem {
    pub entity_id: u64,
    pub canonical_name: String,
    pub primary_translation: String,
    pub kind: LexiconEntityKind,
    pub category: DanbooruCategory,
    pub post_count: u64,
    pub rating: LexiconContentRating,
    pub matched_text: String,
    pub match_reason: LexiconMatchReason,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconSearchFilters {
    pub entity_kinds: Vec<LexiconEntityKind>,
    pub categories: Vec<DanbooruCategory>,
    pub group_ids: Vec<String>,
    pub ratings: Vec<LexiconContentRating>,
}

impl Default for LexiconSearchFilters {
    fn default() -> Self {
        Self {
            entity_kinds: vec![LexiconEntityKind::Tag, LexiconEntityKind::Artist],
            categories: Vec::new(),
            group_ids: Vec::new(),
            ratings: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexiconSearchQuery {
    pub text: String,
    pub mode: LexiconSearchMode,
    pub filters: LexiconSearchFilters,
    pub selected_entity_ids: Vec<u64>,
    pub offset: usize,
    pub limit: usize,
}

impl LexiconSearchQuery {
    /// Validates and normalizes pagination limits.
    ///
    /// # Errors
    /// Returns an error if a request could cause an unbounded query.
    pub fn validate(&self) -> crate::LexiconResult<()> {
        if self.limit == 0 || self.limit > 100 {
            return Err(crate::LexiconError::invalid_request(
                "limit must be between 1 and 100",
            ));
        }
        if self.offset > 10_000 {
            return Err(crate::LexiconError::invalid_request(
                "offset must not exceed 10000",
            ));
        }
        if self.text.chars().count() > 1_024 {
            return Err(crate::LexiconError::invalid_request(
                "query text must not exceed 1024 characters",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexiconSearchPage {
    pub items: Vec<LexiconSearchItem>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalizedLexiconText {
    pub locale: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexiconRelatedEntity {
    pub entity: LexiconSearchItem,
    pub relation: String,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LexiconEntityDetail {
    pub entity: LexiconSearchItem,
    pub translations: Vec<LocalizedLexiconText>,
    pub aliases: Vec<String>,
    pub wiki: Vec<LocalizedLexiconText>,
    pub groups: Vec<LexiconGroupSummary>,
    pub related: Vec<LexiconRelatedEntity>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLexiconEntity {
    pub entity_id: u64,
    pub canonical_name: String,
    pub aliases: Vec<String>,
}
