use atelier_app_api::explore::ExploreQueryDto;
use atelier_explore::{ExploreCursor, ExploreError, ExploreResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use super::CommandResult;

pub(super) fn fingerprint(query: &ExploreQueryDto, revision: u64) -> CommandResult<String> {
    let bytes = serde_json::to_vec(query).map_err(|_| {
        atelier_app_api::error::ErrorEnvelopeDto::new(
            "explore_invalid_request",
            "could not encode query",
        )
    })?;
    Ok(URL_SAFE_NO_PAD.encode(
        Sha256::new()
            .chain_update(bytes)
            .chain_update(revision.to_le_bytes())
            .finalize(),
    ))
}

pub(super) fn encode(cursor: ExploreCursor, fingerprint: &str) -> String {
    let (kind, value) = match cursor {
        ExploreCursor::BeforeId(id) => ("b", id),
        ExploreCursor::Offset(offset) => ("o", offset),
    };
    format!("v1:{fingerprint}:{kind}:{value}")
}

pub(super) fn decode(
    token: Option<&str>,
    fingerprint: &str,
) -> ExploreResult<Option<ExploreCursor>> {
    let Some(token) = token else {
        return Ok(None);
    };
    let invalid =
        || ExploreError::invalid("cursor does not belong to this source, query, or identity");
    if token.len() > 128 {
        return Err(invalid());
    }
    let fields: Vec<_> = token.split(':').collect();
    if fields.len() != 4 || fields[0] != "v1" || fields[1] != fingerprint {
        return Err(invalid());
    }
    let value = fields[3]
        .parse::<u64>()
        .ok()
        .filter(|v| *v > 0)
        .ok_or_else(invalid)?;
    Ok(Some(match fields[2] {
        "b" => ExploreCursor::BeforeId(value),
        "o" => ExploreCursor::Offset(value),
        _ => return Err(invalid()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atelier_app_api::{danbooru::DanbooruRatingDto, explore::DanbooruExploreQueryDto};

    #[test]
    fn cursor_is_bound_to_query_and_identity() {
        let query = ExploreQueryDto::DanbooruDatabase(DanbooruExploreQueryDto {
            query: "sky".into(),
            ratings: vec![DanbooruRatingDto::General],
        });
        let key = fingerprint(&query, 0).unwrap();
        let token = encode(ExploreCursor::BeforeId(42), &key);
        assert_eq!(
            decode(Some(&token), &key).unwrap(),
            Some(ExploreCursor::BeforeId(42))
        );
        assert!(decode(Some(&token), &fingerprint(&query, 1).unwrap()).is_err());
        assert!(decode(Some(&token), "another source").is_err());
        assert!(decode(Some("bad"), &key).is_err());
    }
}
