use unicode_normalization::UnicodeNormalization;

use crate::{ResourceLibraryError, ResourceLibraryErrorKind, ResourceLibraryResult};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceIdentifier(String);

impl ResourceIdentifier {
    /// Parses one NFC-normalized logical path segment.
    ///
    /// # Errors
    /// Returns an error when the value is empty or is not a Unicode identifier
    /// optionally containing hyphens after its first character.
    pub fn parse(value: &str) -> ResourceLibraryResult<Self> {
        let normalized = value.nfc().collect::<String>();
        let mut chars = normalized.chars();
        let valid_start = chars
            .next()
            .is_some_and(|character| character == '_' || unicode_ident::is_xid_start(character));
        let valid_tail = chars.all(|character| {
            character == '_' || character == '-' || unicode_ident::is_xid_continue(character)
        });
        if !valid_start || !valid_tail {
            return Err(ResourceLibraryError::new(
                ResourceLibraryErrorKind::InvalidIdentifier,
                format!("invalid resource identifier `{value}`"),
            ));
        }
        Ok(Self(normalized))
    }

    #[must_use]
    pub fn from_legacy(value: &str, fallback: &str) -> Self {
        let normalized = value.trim().nfc().collect::<String>();
        let mut output = String::new();
        let mut previous_was_replacement = false;
        for character in normalized.chars() {
            let valid = if output.is_empty() {
                character == '_' || unicode_ident::is_xid_start(character)
            } else {
                character == '_' || character == '-' || unicode_ident::is_xid_continue(character)
            };
            if valid {
                output.push(character);
                previous_was_replacement = false;
                continue;
            }
            if output.is_empty() && (character == '-' || unicode_ident::is_xid_continue(character))
            {
                output.push('_');
                output.push(character);
                previous_was_replacement = false;
            } else if !previous_was_replacement {
                output.push('_');
                previous_was_replacement = true;
            }
        }
        let candidate = if output.chars().all(|character| character == '_') {
            fallback
        } else {
            output.as_str()
        };
        Self::parse(candidate).unwrap_or_else(|_| Self("item".to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourcePath(String);

impl ResourcePath {
    /// Parses a non-empty root-relative resource path.
    ///
    /// # Errors
    /// Returns an error for leading, trailing, or repeated separators and when
    /// any path segment is not a valid [`ResourceIdentifier`].
    pub fn parse(value: &str) -> ResourceLibraryResult<Self> {
        if value.is_empty() || value.starts_with('/') || value.ends_with('/') {
            return Err(invalid_path(value));
        }
        let segments = value
            .split('/')
            .map(ResourceIdentifier::parse)
            .collect::<ResourceLibraryResult<Vec<_>>>()?;
        if segments.is_empty() {
            return Err(invalid_path(value));
        }
        Ok(Self(
            segments
                .iter()
                .map(ResourceIdentifier::as_str)
                .collect::<Vec<_>>()
                .join("/"),
        ))
    }

    #[must_use]
    pub fn from_segments<'a>(segments: impl IntoIterator<Item = &'a ResourceIdentifier>) -> Self {
        Self(
            segments
                .into_iter()
                .map(ResourceIdentifier::as_str)
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    #[must_use]
    /// Returns the final identifier segment.
    ///
    /// # Panics
    /// Panics only if an internally constructed path violates the non-empty
    /// invariant established by [`Self::parse`].
    pub fn identifier(&self) -> ResourceIdentifier {
        let value = self
            .0
            .rsplit('/')
            .next()
            .expect("resource path is non-empty");
        ResourceIdentifier::parse(value).expect("resource path segments are validated")
    }
}

fn invalid_path(value: &str) -> ResourceLibraryError {
    ResourceLibraryError::new(
        ResourceLibraryErrorKind::InvalidPath,
        format!("invalid resource path `{value}`"),
    )
}
