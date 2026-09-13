use std::collections::BTreeSet;

use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use crate::{
    ResourceIdentifier, ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryResult,
};

macro_rules! library_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn allocate() -> Self {
                Self(Uuid::new_v4().hyphenated().to_string())
            }

            /// Parses and canonicalizes a library UUID.
            ///
            /// # Errors
            /// Returns an error when `value` is not a UUID.
            pub fn parse(value: &str) -> ResourceLibraryResult<Self> {
                let parsed = Uuid::parse_str(value).map_err(|_| {
                    ResourceLibraryError::new(
                        ResourceLibraryErrorKind::InvalidIdentifier,
                        format!("invalid {} `{value}`", $label),
                    )
                })?;
                Ok(Self(parsed.hyphenated().to_string()))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

library_id!(LibraryResourceId, "library resource id");
library_id!(LibraryFolderId, "library folder id");

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LibraryNamespace {
    PromptChunk,
    MainPreset,
    CharacterPreset,
    Vibe,
}

impl LibraryNamespace {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PromptChunk => "prompt_chunk",
            Self::MainPreset => "main_preset",
            Self::CharacterPreset => "character_preset",
            Self::Vibe => "vibe",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceName {
    pub identifier: ResourceIdentifier,
    pub display_name: String,
    pub aliases: Vec<String>,
}

impl ResourceName {
    /// Creates normalized user-facing naming metadata.
    ///
    /// A blank display name falls back to the identifier.
    ///
    /// # Errors
    /// Returns an error when neither value provides a visible name.
    pub fn new(
        identifier: ResourceIdentifier,
        display_name: &str,
        aliases: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> ResourceLibraryResult<Self> {
        let display_name = normalize_visible_name(display_name, identifier.as_str())?;
        let mut seen = BTreeSet::new();
        let aliases = aliases
            .into_iter()
            .filter_map(|alias| {
                let normalized = alias.as_ref().trim().nfc().collect::<String>();
                (!normalized.is_empty()
                    && normalized != display_name
                    && seen.insert(normalized.clone()))
                .then_some(normalized)
            })
            .collect();
        Ok(Self {
            identifier,
            display_name,
            aliases,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryFolder {
    pub id: LibraryFolderId,
    pub namespace: LibraryNamespace,
    pub parent_id: Option<LibraryFolderId>,
    pub identifier: ResourceIdentifier,
    pub display_name: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl LibraryFolder {
    /// Creates a normalized folder.
    ///
    /// A blank display name falls back to the identifier.
    ///
    /// # Errors
    /// Returns an error when neither value provides a visible name.
    pub fn new(
        id: LibraryFolderId,
        namespace: LibraryNamespace,
        parent_id: Option<LibraryFolderId>,
        identifier: ResourceIdentifier,
        display_name: &str,
        now_ms: u64,
    ) -> ResourceLibraryResult<Self> {
        let display_name = normalize_visible_name(display_name, identifier.as_str())?;
        Ok(Self {
            id,
            namespace,
            parent_id,
            identifier,
            display_name,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryResource {
    pub id: LibraryResourceId,
    pub namespace: LibraryNamespace,
    pub folder_id: Option<LibraryFolderId>,
    pub name: ResourceName,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

fn normalize_visible_name(value: &str, fallback: &str) -> ResourceLibraryResult<String> {
    let normalized = value.trim().nfc().collect::<String>();
    let normalized = if normalized.is_empty() {
        fallback.trim().nfc().collect()
    } else {
        normalized
    };
    if normalized.is_empty() {
        return Err(ResourceLibraryError::new(
            ResourceLibraryErrorKind::InvalidName,
            "resource display name and fallback cannot both be empty",
        ));
    }
    Ok(normalized)
}
