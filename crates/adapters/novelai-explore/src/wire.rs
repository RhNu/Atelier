use atelier_explore::{
    ExploreError, ExploreErrorKind, ExploreResult,
    novelai::{
        NovelAiExplorePeriod, NovelAiExplorePost, NovelAiExploreQuery, NovelAiExploreSort,
        validate_post_id,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::metadata::parse_metadata;

pub const PAGE_SIZE: u64 = 40;

pub fn search_body(query: &NovelAiExploreQuery, offset: u64) -> Value {
    let field = match query.sort {
        NovelAiExploreSort::New => "created_at",
        NovelAiExploreSort::Top => "top",
        NovelAiExploreSort::Hot => "hot",
        NovelAiExploreSort::Random => "random",
    };
    let period = match query.period {
        Some(NovelAiExplorePeriod::Day) => "day",
        Some(NovelAiExplorePeriod::Month) => "month",
        _ => "week",
    };
    let selectors = if query.sort == NovelAiExploreSort::Random {
        vec![
            json!({"field":"random", "value":format!("{period}:{}", query.random_salt.as_deref().unwrap_or_default())}),
        ]
    } else {
        let mut selectors: Vec<Value> = query
            .tags
            .iter()
            .map(|tag| json!({"field":"tag","value":tag.trim()}))
            .collect();
        if query.sort == NovelAiExploreSort::New {
            selectors.push(json!({"field":"moderation_status","value":"1"}));
        } else {
            selectors.push(json!({"field":field,"value":period}));
        }
        if let Some(id) = &query.creator_id {
            selectors.push(json!({"field":"creator_id","value":id}));
        }
        selectors
    };
    json!({"orderers":[{"field":field,"sort_direction":"desc"}], "selectors":selectors,
        "pagination":{"limit":PAGE_SIZE,"offset":offset}})
}

#[derive(Deserialize)]
pub struct RawPage {
    pub results: Vec<Value>,
    pub pagination: RawPagination,
}

#[derive(Deserialize)]
pub struct RawPagination {
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}

#[derive(Deserialize)]
struct RawPost {
    id: String,
    #[serde(rename = "type")]
    post_type: u32,
    moderation_status: u32,
    deleted: bool,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    creator_id: Option<String>,
    #[serde(default)]
    creator: Option<RawCreator>,
    created_at: String,
    #[serde(default)]
    like_count: Option<u64>,
    image: RawImage,
}

#[derive(Deserialize)]
struct RawCreator {
    id: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize)]
struct RawImage {
    width: u32,
    height: u32,
    nai_metadata: Option<Value>,
}

pub fn decode_post(value: Value) -> ExploreResult<NovelAiExplorePost> {
    let post: RawPost = serde_json::from_value(value).map_err(|_| {
        ExploreError::new(
            ExploreErrorKind::InvalidResponse,
            "NovelAI Explore post shape changed",
        )
    })?;
    if post.deleted || post.moderation_status != 1 || post.post_type != 1 {
        return Err(ExploreError::new(
            ExploreErrorKind::NotFound,
            "Explore post is not a public approved image",
        ));
    }
    validate_post_id(&post.id).map_err(|_| {
        ExploreError::new(
            ExploreErrorKind::InvalidResponse,
            "invalid Explore post identifier",
        )
    })?;
    Ok(NovelAiExplorePost {
        id: post.id,
        title: post.title,
        description: post.description,
        creator_id: post
            .creator_id
            .or_else(|| post.creator.as_ref().and_then(|c| c.id.clone())),
        creator_name: post.creator.and_then(|c| c.name),
        created_at: post.created_at,
        width: post.image.width,
        height: post.image.height,
        like_count: post.like_count,
        metadata: parse_metadata(post.image.nai_metadata.as_ref()),
    })
}
